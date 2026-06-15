use wgpu::RenderPipeline;

pub mod grid_pipeline;

pub trait Pipeline {
    fn create_pipeline(
        &self,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> RenderPipeline;
}
