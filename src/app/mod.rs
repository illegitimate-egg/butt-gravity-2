#[cfg(target_arch = "wasm32")]
use std::fmt::UpperHex;
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use winit::event_loop::EventLoop;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use winit::{application::ApplicationHandler, event::{KeyEvent, WindowEvent}, event_loop::{ActiveEventLoop, OwnedDisplayHandle}, keyboard::{KeyCode, PhysicalKey}, window::Window};

use crate::renderer::{Renderer, texture::Texture};

pub struct State {
    renderer: Renderer,
    is_surface_configured: bool,
    window: Arc<Window>,
}

impl State {
    pub async fn new(window: Arc<Window>, display_handle: Box<OwnedDisplayHandle>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::Primary,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            // GL is almost esoteric sometimes
            // I'm not even sure if this is worth doing. gl still doesn't work on my desktop with this and wasm seems to not give a shit
            display: Some(display_handle),
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            // dgpu so we can suckle down all the coulombs
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await?;

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
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let egui_context = egui::Context::default();
        let egui_state = egui_winit::State::new(egui_context, egui::viewport::ViewportId::ROOT, &window, Some(1.0), None, Some(2048));
        let egui_renderer = egui_wgpu::Renderer::new(&device, surface_format, egui_wgpu::RendererOptions { ..Default::default() });

        // Force the initial surface creation to bind immediately to the active X11 window tree
        surface.configure(&device, &config);
               
        Ok(Self {
            renderer: Renderer::new(surface, device, queue, config, egui_renderer, egui_state),
            is_surface_configured: true, // Buttily needed for x11
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
            self.renderer.depth_texture = Texture::create_depth_texture(&self.renderer.device, &self.renderer.config, "depth_texture");
            self.is_surface_configured = true;
        }
    }

    fn update(&mut self) {}

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        self.renderer.render(self.window.clone())
    }

    fn handle_key(&self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
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

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        let _ = state.renderer.egui_state.on_window_event(&state.window, &event);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
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
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            _ => {}
        }
    }
}
