use std::ops::Deref;

use crate::WgpuContext;

pub mod simple;

pub trait PipelineVertex: bytemuck::Pod + bytemuck::Zeroable {
    type Pipeline: RenderPipeline;

    fn pipeline_id() -> std::any::TypeId {
        std::any::TypeId::of::<Self::Pipeline>()
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

pub trait RenderPipeline: Deref<Target = wgpu::RenderPipeline> {
    type Vertex: PipelineVertex;
    type Uniforms: bytemuck::Pod + bytemuck::Zeroable;
    fn new(ctx: &WgpuContext) -> Self
    where
        Self: Sized;
}
