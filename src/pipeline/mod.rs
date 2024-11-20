use std::ops::Deref;

use glam::{Vec3, Vec4};

use crate::WgpuContext;

pub mod simple;

pub trait PipelineVertex: bytemuck::Pod + bytemuck::Zeroable {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;

    fn position(&self) -> Vec3;
    fn set_position(&mut self, position: Vec3);

    fn color(&self) -> Vec4;
    fn set_color(&mut self, color: Vec4);

    fn interpolate(&self, other: &Self, t: f32) -> Self;
}

/// A render pipeline.
pub trait RenderPipeline: Deref<Target = wgpu::RenderPipeline> {
    /// The vertex type.
    type Vertex: PipelineVertex;
    /// The uniform type.
    type Uniforms: bytemuck::Pod + bytemuck::Zeroable;
    fn new(ctx: &WgpuContext) -> Self
    where
        Self: Sized;
}
