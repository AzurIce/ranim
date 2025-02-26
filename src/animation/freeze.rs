use super::{AnimSchedule, EntityAnim, StaticEntityAnim};
use crate::{
    items::{Entity, Rabject},
    render::{RenderTextures, Renderable, StaticRenderable},
};

pub trait FreezeAnim<'r, 't, T: Entity + 'static> {
    fn freeze(&'r mut self) -> AnimSchedule<'r, 't, T, EntityAnim<T>>;
}

impl<'r, 't, T: Entity + 'static> FreezeAnim<'r, 't, T> for Rabject<'t, T> {
    fn freeze(&'r mut self) -> AnimSchedule<'r, 't, T, EntityAnim<T>> {
        AnimSchedule::new(self, StaticEntityAnim::new(self.id, self.data.clone()))
    }
}

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

impl StaticRenderable for Blank {
    fn prepare(
        &self,
        _ctx: &crate::context::WgpuContext,
        _render_instances: &mut crate::render::primitives::RenderInstances,
    ) {
        // DO NOTHING
    }
}
