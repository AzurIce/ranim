use std::{fmt::Debug, ops::Deref};

use crate::RanimContext;

/// A render pipeline.
pub trait RenderPipeline: Deref<Target = wgpu::RenderPipeline> {
    /// The vertex type.
    ///
    /// used to define the vertex format.
    type Vertex: PipelineVertex;

    fn new(ctx: &RanimContext) -> Self
    where
        Self: Sized;
}

pub trait PipelineVertex: bytemuck::Pod + bytemuck::Zeroable + Clone + Debug {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}
