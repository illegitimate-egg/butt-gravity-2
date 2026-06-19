use wgpu::{BindGroup, TextureView};

use crate::renderer::render_passes::RenderPass;

pub struct BodyDrawPass<'a> {
    pub bodies_bind_group: &'a BindGroup,
    pub camera_bind_group: &'a BindGroup,
    pub depth_texture_view: &'a TextureView,
    pub body_count: u32,
}

impl RenderPass for BodyDrawPass<'_> {
    fn render_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        resolve_target: Option<&TextureView>,
        pipeline: &wgpu::RenderPipeline,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Body Draw Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, self.bodies_bind_group, &[]);
        pass.set_bind_group(1, self.camera_bind_group, &[]);

        pass.draw(0..6, 0..self.body_count);
    }
}
