use crate::{WgpuBuffer, WgpuContext};

pub mod simple;

pub trait RenderPipeline {
    type Vertex: bytemuck::Pod + bytemuck::Zeroable;
    type Uniforms: bytemuck::Pod + bytemuck::Zeroable;
    fn new(ctx: &WgpuContext) -> Self where Self: Sized;
    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        depth_view: Option<&wgpu::TextureView>,
        vertex_buffer: &WgpuBuffer<Self::Vertex>,
        bindgroups: &[&wgpu::BindGroup],
    );
}
