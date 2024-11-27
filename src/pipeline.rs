use std::{fmt::Debug, ops::Deref};

use crate::RanimContext;

/// A pipeline.
pub trait Pipeline {
    fn new(ctx: &RanimContext) -> Self
    where
        Self: Sized;
}

pub trait Vertex: bytemuck::Pod + bytemuck::Zeroable + Clone + Debug {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}
