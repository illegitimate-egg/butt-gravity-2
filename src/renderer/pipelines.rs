use wgpu::RenderPipeline;

pub mod body_draw_pipeline;
pub mod grid_pipeline;

pub trait Pipeline {
    fn create_pipeline(
        &self,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> RenderPipeline;
}
