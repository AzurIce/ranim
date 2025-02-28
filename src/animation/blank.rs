use crate::render::{RenderTextures, Renderable, StaticEval};

pub struct Blank;

impl Renderable for Blank {
    fn render(
        &self,
        _ctx: &crate::context::WgpuContext,
        _render_instances: &mut crate::render::primitives::RenderInstances,
        _pipelines: &mut crate::utils::PipelinesStorage,
        _encoder: &mut wgpu::CommandEncoder,
        _uniforms_bind_group: &wgpu::BindGroup,
        _render_textures: &RenderTextures,
    ) {
        // DO NOTHING
    }
}

impl StaticEval for Blank {
    fn prepare(
        &self,
        _ctx: &crate::context::WgpuContext,
        _render_instances: &mut crate::render::primitives::RenderInstances,
    ) {
        // DO NOTHING
    }
}
