use egui::Context;
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::State;

pub struct EguiPass<'a> {
    pub window: &'a std::sync::Arc<winit::window::Window>,
    pub egui_state: &'a mut State,
    pub egui_renderer: &'a mut Renderer,
}

impl EguiPass<'_> {
    pub fn begin_ui(&mut self) {
        let raw_input = self.egui_state.take_egui_input(self.window);
        self.egui_state.egui_ctx().begin_pass(raw_input);
    }

    pub fn get_ctx(&mut self) -> &Context {
        self.egui_state.egui_ctx()
    }

    pub fn end_ui(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) {
        let full_output = self.egui_state.egui_ctx().end_pass();

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [config.width, config.height],
            pixels_per_point: 1.0,
        };

        self.egui_state
            .handle_platform_output(self.window, full_output.platform_output);

        let tris = self.egui_state.egui_ctx().tessellate(
            full_output.shapes,
            self.egui_state.egui_ctx().pixels_per_point(),
        );
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(device, queue, *id, image_delta);
        }
        self.egui_renderer
            .update_buffers(device, queue, encoder, &tris, &screen_descriptor);

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui main pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
}
