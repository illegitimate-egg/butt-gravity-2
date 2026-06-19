use egui::DragValue;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use glam::DQuat;
use log::info;
use wgpu::Backend;
use wgpu::{util::DeviceExt, TextureView};

#[cfg(target_arch = "wasm32")]
use crate::app::CANVAS_ID;

use crate::renderer::compute_passes::step_a_y4::{BodyState, Y4AccelerationStep};
use crate::renderer::phys_state::PhysState;
use crate::renderer::pipelines::body_draw_pipeline::BodyDrawPipeline;
use crate::renderer::render_passes::body_draw_pass::BodyDrawPass;
use crate::renderer::render_passes::egui_pass::EguiPass;
use crate::renderer::{
    camera::{Camera, CameraUniform},
    pipelines::{grid_pipeline::GridPipeline, Pipeline},
    render_passes::{grid_pass::GridPass, RenderPass},
    texture::{create_msaa, Texture},
};

pub mod camera;
mod compute_passes;
mod phys_state;
mod pipelines;
mod render_passes;
pub mod texture;

// Should be either 1 or 4
const MSAA_SAMPLES: u32 = 4;

pub fn get_msaa_samples(device: &wgpu::Device) -> u32 {
    if device.adapter_info().backend == Backend::Gl {
        1
    } else {
        MSAA_SAMPLES
    }
}

pub struct Renderer {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    pub last_frame_instant: Instant,
    pub last_frame_time: f32,
    pub delta_time: f32,

    pub camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    depth_texture: Texture,
    msaa_texture: wgpu::Texture,
    msaa_view: wgpu::TextureView,

    max_view_dimensions: (u32, u32),

    simulator: Y4AccelerationStep,
    phys_state: PhysState,

    grid_pipeline: wgpu::RenderPipeline,
    body_draw_pipeline: wgpu::RenderPipeline,

    pub egui_renderer: egui_wgpu::Renderer,
    pub egui_state: egui_winit::State,
}

impl Renderer {
    pub fn new(
        surface: wgpu::Surface<'static>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: wgpu::SurfaceConfiguration,
        egui_renderer: egui_wgpu::Renderer,
        egui_state: egui_winit::State,
    ) -> Renderer {
        #[cfg(not(target_arch = "wasm32"))]
        let max_view_dimensions = (
            device.limits().max_texture_dimension_2d,
            device.limits().max_texture_dimension_2d,
        );
        #[cfg(target_arch = "wasm32")]
        let max_view_dimensions: (u32, u32) = if device.adapter_info().backend == Backend::Gl {
            use wasm_bindgen::prelude::*;
            use wasm_bindgen::JsCast;

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element: web_sys::HtmlCanvasElement = canvas.unchecked_into();

            // Minimum guaranteed by spec
            let mut max_width: u32 = 2048;
            let mut max_height: u32 = 2048;

            if let Some(context_obj) = html_canvas_element.get_context("webgl2").unwrap_throw() {
                let gl: wgpu::web_sys::WebGl2RenderingContext = context_obj.unchecked_into();

                if let Ok(dims_val) =
                    gl.get_parameter(web_sys::WebGl2RenderingContext::MAX_VIEWPORT_DIMS)
                {
                    if let Some(array) = dims_val.dyn_ref::<js_sys::Int32Array>() {
                        let mut max_dims = [0i32; 2];
                        array.copy_to(&mut max_dims);

                        max_width = max_dims[0] as u32;
                        max_height = max_dims[1] as u32;
                    }
                }
            }
            (max_width, max_height)
        } else {
            (
                device.limits().max_texture_dimension_2d,
                device.limits().max_texture_dimension_2d,
            )
        };

        info!(
            "Maximum size {}x{}",
            max_view_dimensions.0, max_view_dimensions.1
        );

        let samples = get_msaa_samples(&device);

        if samples == 1 {
            info!("MSAA Disabled");
        } else {
            info!("Using {}x MSAA", samples);
        }

        let depth_texture = Texture::create_depth_texture(
            &device,
            &config,
            "depth_texture",
            get_msaa_samples(&device),
        );
        let (msaa_texture, msaa_view) =
            create_msaa(&device, &config, "msaa_color", get_msaa_samples(&device));

        let last_frame_instant = Instant::now();
        let last_frame_time = 0.0;
        let delta_time = 0.0;

        let camera = Camera {
            position: (1.0, 1.0, 1.0).into(),
            orientation: DQuat::IDENTITY,
            fov_y: 90.0_f64.to_radians(),
            near_plane: 0.1,
            far_plane: 1e7,
        };

        let camera_uniform = CameraUniform {
            view: camera.view_matrix_single().to_cols_array(),
            view_proj: camera
                .view_projection_matrix_single(config.width as f64 / config.height as f64)
                .to_cols_array(),
        };

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera V VP Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let simulator = Y4AccelerationStep::new(
            &device,
            vec![
                BodyState {
                    position_radius: [0.9700436, -0.24308753, 0.0, 0.1],
                    velocity_mass: [0.4662037, 0.43236573, 0.0, 1.498e10],
                    color: [1.0, 0.0, 0.0, 1.0],
                },
                BodyState {
                    position_radius: [-0.9700436, 0.24308753, 0.0, 0.1],
                    velocity_mass: [0.4662037, 0.43236573, 0.0, 1.498e10],
                    color: [0.0, 1.0, 0.0, 1.0],
                },
                BodyState {
                    position_radius: [0.0, 0.0, 0.0, 0.1],
                    velocity_mass: [-2.0 * 0.4662037, -2.0 * 0.43236573, 0.0, 1.498e10],
                    color: [0.0, 0.0, 1.0, 1.0],
                },
            ],
        );

        let mut phys_state = PhysState::new();
        phys_state.traversal_modifier = 0.00;

        // Build pipelines
        let grid_pipeline = GridPipeline {
            camera_bind_group_layout: &camera_bind_group_layout,
        }
        .create_pipeline(&device, &config);

        let body_draw_pipeline = BodyDrawPipeline {
            bodies_bind_group_layout: &simulator.compute_layout,
            camera_bind_group_layout: &camera_bind_group_layout,
        }
        .create_pipeline(&device, &config);

        Renderer {
            surface,
            device,
            queue,
            config,
            last_frame_instant,
            last_frame_time,
            delta_time,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            depth_texture,
            msaa_texture,
            msaa_view,
            max_view_dimensions,
            simulator,
            phys_state,
            grid_pipeline,
            body_draw_pipeline,
            egui_renderer,
            egui_state,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.min(self.max_view_dimensions.0);
        self.config.height = height.min(self.max_view_dimensions.1);

        self.surface.configure(&self.device, &self.config);

        self.depth_texture = Texture::create_depth_texture(
            &self.device,
            &self.config,
            "depth_texture",
            get_msaa_samples(&self.device),
        );
        (self.msaa_texture, self.msaa_view) = create_msaa(
            &self.device,
            &self.config,
            "msaa_color",
            get_msaa_samples(&self.device),
        );
    }

    pub fn render(&mut self, window: &std::sync::Arc<winit::window::Window>) -> anyhow::Result<()> {
        self.delta_time = Instant::now()
            .duration_since(self.last_frame_instant)
            .as_secs_f32();
        self.last_frame_instant = Instant::now();

        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                self.surface.configure(&self.device, &self.config);
                surface_texture
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                anyhow::bail!("Lost device");
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let use_view: &TextureView;
        let resolve_target: Option<&TextureView>;

        if get_msaa_samples(&self.device) > 1 {
            use_view = &self.msaa_view;
            resolve_target = Some(&view);
        } else {
            use_view = &view;
            resolve_target = None;
        }

        GridPass {
            camera_bind_group: &self.camera_bind_group,
            depth_texture_view: &self.depth_texture.view,
        }
        .render_pass(&mut encoder, use_view, resolve_target, &self.grid_pipeline);

        self.phys_state
            .cycle(self.delta_time, self.simulator.sim_params.dt, || {
                self.simulator.step_simulation(&mut encoder);
            });

        let body_bind_group = if !self.simulator.swap_buffers {
            &self.simulator.ab_bind_group
        } else {
            &self.simulator.ba_bind_group
        };

        BodyDrawPass {
            bodies_bind_group: body_bind_group,
            camera_bind_group: &self.camera_bind_group,
            depth_texture_view: &self.depth_texture.view,
            body_count: self.simulator.sim_params.body_count,
        }
        .render_pass(
            &mut encoder,
            use_view,
            resolve_target,
            &self.body_draw_pipeline,
        );

        {
            // Whole egui loop right here
            // Pre-ui
            let mut egui_pass = EguiPass {
                window,
                egui_state: &mut self.egui_state,
                egui_renderer: &mut self.egui_renderer,
            };
            egui_pass.begin_ui();

            // Ui
            egui::Window::new("Camera information")
                .resizable(true)
                .vscroll(true)
                .show(egui_pass.get_ctx(), |ui| {
                    ui.label(format!(
                        "Delta Time: {}\nFrame Time: {}\nFPS: {}\nEngine Debt: {}",
                        self.delta_time,
                        self.last_frame_time,
                        self.delta_time.recip(),
                        self.phys_state.get_debt(),
                    ));
                    ui.label("Position (XYZ)");
                    ui.columns(3, |ui| {
                        ui[0].add(egui::DragValue::new(&mut self.camera.position.x));
                        ui[1].add(egui::DragValue::new(&mut self.camera.position.y));
                        ui[2].add(egui::DragValue::new(&mut self.camera.position.z));
                    });
                    ui.label("QRot (XYZW)");
                    ui.columns(4, |ui| {
                        ui[0].add(egui::DragValue::new(&mut self.camera.orientation.x));
                        ui[1].add(egui::DragValue::new(&mut self.camera.orientation.y));
                        ui[2].add(egui::DragValue::new(&mut self.camera.orientation.z));
                        ui[3].add(egui::DragValue::new(&mut self.camera.orientation.w));
                    });
                    let euler_camera = self.camera.orientation.to_euler(glam::EulerRot::XYZ);
                    ui.label("ERot (XYZ)");
                    ui.columns(3, |ui| {
                        ui[0].drag_angle(&mut (euler_camera.0 as f32));
                        ui[1].drag_angle(&mut (euler_camera.1 as f32));
                        ui[2].drag_angle(&mut (euler_camera.2 as f32));
                    });
                    ui.label("Time modifier:");
                    ui.add(
                        DragValue::new(&mut self.phys_state.traversal_modifier)
                            .speed(0.01)
                            .suffix("x")
                            .range(0.0..=f32::MAX),
                    );
                });

            // Post-ui
            egui_pass.end_ui(&mut encoder, &view, &self.device, &self.queue, &self.config);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.last_frame_time = Instant::now()
            .duration_since(self.last_frame_instant)
            .as_secs_f32();

        Ok(())
    }

    pub fn update_camera(&mut self) {
        self.camera_uniform.view = self.camera.view_matrix_single().to_cols_array();
        self.camera_uniform.view_proj = self
            .camera
            .view_projection_matrix_single(self.config.width as f64 / self.config.height as f64)
            .to_cols_array();
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }
}
