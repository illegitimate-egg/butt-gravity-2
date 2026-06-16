use egui_wgpu::ScreenDescriptor;
use glam::{DQuat, DVec3};
use wgpu::util::DeviceExt;

use crate::renderer::{
    camera::{Camera, CameraUniform},
    pipelines::{grid_pipeline::GridPipeline, Pipeline},
    render_passes::{grid_pass::GridPass, RenderPass},
    texture::Texture,
};

pub mod camera;
mod pipelines;
mod render_passes;
pub mod texture;

pub struct Renderer {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    pub camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    pub depth_texture: Texture,

    grid_pipeline: wgpu::RenderPipeline,
    // line_pipeline: wgpu::RenderPipeline,
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
        let depth_texture = Texture::create_depth_texture(&device, &config, "depth_texture");

        let camera = Camera {
            position: (0.0, 0.0, 1.0).into(),
            orientation: DQuat::IDENTITY,
            fov_y: 90.0_f64.to_radians(),
            near_plane: 0.1,
            far_plane: 1e7,
        };

        let camera_uniform = CameraUniform {
            view_proj: camera
                .view_projection_matrix_single(config.width as f64 / config.height as f64)
                .to_cols_array(),
        };

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera VP Buffer"),
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

        // Build pipelines
        let grid_pipeline = GridPipeline {
            camera_bind_group_layout,
        }
        .create_pipeline(&device, &config);

        Renderer {
            surface,
            device,
            queue,
            config,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            depth_texture,
            grid_pipeline,
            egui_renderer,
            egui_state,
        }
    }

    pub fn render(&mut self, window: std::sync::Arc<winit::window::Window>) -> anyhow::Result<()> {
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

        GridPass {
            camera_bind_group: &self.camera_bind_group,
            depth_texture_view: &self.depth_texture.view,
        }
        .render_pass(&mut encoder, &view, &self.grid_pipeline);

        {
            // Whole egui loop right here
            // Pre-ui
            let raw_input = self.egui_state.take_egui_input(&window);
            self.egui_state.egui_ctx().begin_pass(raw_input);

            // Ui
            egui::Window::new("Camera information")
                .resizable(true)
                .vscroll(true)
                .show(self.egui_state.egui_ctx(), |ui| {
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
                });

            // Post-ui
            let full_output = self.egui_state.egui_ctx().end_pass();

            let screen_descriptor = ScreenDescriptor {
                size_in_pixels: [self.config.width, self.config.height],
                pixels_per_point: 1.0,
            };

            self.egui_state
                .handle_platform_output(&window, full_output.platform_output);

            let tris = self.egui_state.egui_ctx().tessellate(
                full_output.shapes,
                self.egui_state.egui_ctx().pixels_per_point(),
            );
            for (id, image_delta) in &full_output.textures_delta.set {
                self.egui_renderer
                    .update_texture(&self.device, &self.queue, *id, image_delta);
            }
            self.egui_renderer.update_buffers(
                &self.device,
                &self.queue,
                &mut encoder,
                &tris,
                &screen_descriptor,
            );

            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui main pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            self.egui_renderer.render(
                &mut render_pass.forget_lifetime(),
                &tris,
                &screen_descriptor,
            );
            for x in &full_output.textures_delta.free {
                self.egui_renderer.free_texture(x);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn update_camera(&mut self) {
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
