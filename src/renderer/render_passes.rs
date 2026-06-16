use wgpu::{CommandEncoder, RenderPipeline, TextureView};

pub mod grid_pass;

pub trait RenderPass {
    fn render_pass(
        &self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        resolve_target: Option<&TextureView>,
        pipeline: &RenderPipeline,
    );
}
