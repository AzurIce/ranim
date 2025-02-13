use super::{AnimScheduler, Animator, EntityAnim};
use crate::{
    items::{Entity, Rabject},
    render::{RenderTextures, Renderable},
};

pub trait FreezeAnim<'r, 't, T: Entity + 'static> {
    fn freeze(&'r mut self) -> AnimScheduler<'r, 't, T, EntityAnim<T>>;
}

impl<'r, 't, T: Entity + 'static> FreezeAnim<'r, 't, T> for Rabject<'t, T> {
    fn freeze(&'r mut self) -> AnimScheduler<'r, 't, T, EntityAnim<T>> {
        let data = self.data.clone();
        AnimScheduler::new(self, EntityAnim::new(self.id, self.data.clone(), data))
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
        _camera: &crate::render::CameraFrame,
    ) {
        // DO NOTHING
    }
}
impl Animator for Blank {
    fn update_alpha(&mut self, _alpha: f32) {
        // DO NOTHING
    }
}
