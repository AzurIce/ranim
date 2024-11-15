use crate::{WgpuBuffer, WgpuContext};

pub mod simple;

pub trait Pipeline {
    type Vertex: bytemuck::Pod + bytemuck::Zeroable;

    fn new(ctx: &WgpuContext) -> Self;
    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        vertex_buffer: &WgpuBuffer<Self::Vertex>,
    );
}
