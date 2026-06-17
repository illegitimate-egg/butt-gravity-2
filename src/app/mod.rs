use std::sync::Arc;

use log::{info, warn};
#[cfg(target_arch = "wasm32")]
use winit::event_loop::EventLoop;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use winit::{application::ApplicationHandler, event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent}, event_loop::{ActiveEventLoop, OwnedDisplayHandle}, keyboard::{KeyCode, PhysicalKey}, window::Window};

use crate::{app::camera_controller::CameraController, renderer::Renderer};

mod camera_controller;

pub struct State {
    renderer: Renderer,
    camera_controller: CameraController,
    is_surface_configured: bool,
    window: Arc<Window>,
}

impl State {
    pub async fn new(window: Arc<Window>, display_handle: Box<OwnedDisplayHandle>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::all(),
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            // GL is almost esoteric sometimes
            // I'm not even sure if this is worth doing. gl still doesn't work on my desktop with this and wasm seems to not give a shit
            display: Some(display_handle),
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            // dgpu so we can suckle down all the coulombs
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await?;

        let adapter_info = adapter.get_info();
        
        info!("Adapter info:");
        info!("Adapter: {} [{}] using {}", adapter_info.name, adapter_info.backend, adapter_info.driver);
        if adapter_info.backend == wgpu::Backend::Gl {
            warn!("You are using GL. GL support is very flakey. You have been warned. Capricious is the zephyr of a secondary target.");
        } else if adapter_info.backend == wgpu::Backend::BrowserWebGpu {
            info!("You are using WebGPU!");
        }

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                // Wasm is such a pain in the arse, it does't support all of wgpu's features so we have to knock some off
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            }).await?;

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            // Keep things happy if the size is 0
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let egui_context = egui::Context::default();
        let egui_state = egui_winit::State::new(egui_context, egui::viewport::ViewportId::ROOT, &window, Some(1.0), None, Some(2048));
        let egui_renderer = egui_wgpu::Renderer::new(&device, surface_format, egui_wgpu::RendererOptions { ..Default::default() });

        // Force the initial surface creation to bind immediately to the active X11 window tree
        #[cfg(not(target_arch = "wasm32"))]
        surface.configure(&device, &config);
               
        Ok(Self {
            renderer: Renderer::new(surface, device, queue, config, egui_renderer, egui_state),
            camera_controller: CameraController::new(0.05, 0.002),
            #[cfg(not(target_arch = "wasm32"))]
            is_surface_configured: true, // Buttily needed for x11
            #[cfg(target_arch = "wasm32")]
            is_surface_configured: false,
            window,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            // More WebGL bs
            let max = 2048;
            self.renderer.config.width = width.min(max);
            self.renderer.config.height = height.min(max);
            self.renderer.surface.configure(&self.renderer.device, &self.renderer.config);
            self.renderer.resize();
            self.is_surface_configured = true;
        }
    }

    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.renderer.camera);
        self.renderer.update_camera();
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        self.renderer.render(self.window.clone())
    }

    fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        if code == KeyCode::Escape && is_pressed {
            event_loop.exit();
        } else {
            self.camera_controller.handle_key(code, is_pressed);
        }
    }

    fn handle_click(&mut self, _event_loop: &ActiveEventLoop, element_state: ElementState, button: MouseButton) {
        self.camera_controller.handle_click(element_state, button, self.window.clone());
    }

    fn handle_mouse_motion(&mut self, delta: (f64, f64)) {
        self.camera_controller.handle_mouse_motion(delta);
    }
}

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
}

impl App {
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
        }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        let display_handle = Box::new(event_loop.owned_display_handle());

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.state = Some(pollster::block_on(State::new(window, display_handle)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(proxy
                        .send_event(
                            State::new(window, display_handle)
                                .await
                                .expect("Unable to create canvas")
                        )
                        .is_ok()) 
                });
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        self.state = Some(event);
    }

    fn device_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _device_id: winit::event::DeviceId,
            event: DeviceEvent,
        ) {
        
        let astate = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            DeviceEvent::MouseMotion { delta } => {
                astate.handle_mouse_motion(delta);
            }
            _ => {}
        }
    }
    
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent
    ) {
        let astate = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        // Egui can have it whole
        if astate.renderer.egui_state.on_window_event(&astate.window, &event).consumed {
            return;
        }
        
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => astate.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                astate.update();
                match astate.render() {
                    Ok(_) => {},
                    Err(e) => {
                        log::error!("{e}");
                        event_loop.exit();
                    }
                };
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => astate.handle_key(event_loop, code, key_state.is_pressed()),
            WindowEvent::MouseInput { state, button, .. } => {
                astate.handle_click(event_loop, state, button);
            }
            _ => {}
        }
    }
}
