pub mod vmobject;

use std::fmt::Debug;

use glam::{Vec3, Vec4};

use crate::{
    mobject::ExtractedMobject,
    rabject::{ExtractedRabjectWithId, Rabject, RabjectWithId},
    RanimContext,
};

pub trait Renderer: 'static {
    type RenderResource;
    type Vertex: RendererVertex;

    fn begin_pass<'a>(
        encoder: &'a mut wgpu::CommandEncoder,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
    ) -> wgpu::RenderPass<'a>;

    fn prepare(ctx: &mut RanimContext, mobjects: &mut Vec<&mut ExtractedMobject<Self::Vertex>>);

    fn render<'a>(
        ctx: &mut RanimContext,
        pass: &mut wgpu::RenderPass<'a>,
        mobjects: &mut Vec<&mut ExtractedMobject<Self::Vertex>>,
    );
}

pub trait RendererVertex: bytemuck::Pod + bytemuck::Zeroable + Default + Debug {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;

    fn position(&self) -> Vec3;
    fn set_position(&mut self, position: Vec3);

    fn color(&self) -> Vec4;
    fn set_color(&mut self, color: Vec4);

    fn interpolate(&self, other: &Self, t: f32) -> Self;
}
