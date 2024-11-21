use std::ops::Deref;

use crate::{renderer::RendererVertex, RanimContext};

pub mod simple;


/// A render pipeline.
pub trait RenderPipeline: Deref<Target = wgpu::RenderPipeline> {
    /// The vertex type.
    /// 
    /// used to define the vertex format.
    type Vertex: RendererVertex + Clone;

    fn new(ctx: &RanimContext) -> Self
    where
        Self: Sized;
}
