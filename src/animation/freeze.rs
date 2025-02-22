use super::{AnimWithParams, Animator, EntityAnim};
use crate::{
    items::{Entity, Rabject},
    render::{RenderTextures, Renderable},
};

pub fn freeze<T: Entity + 'static>(rabject: &Rabject<T>) -> AnimWithParams<EntityAnim<T>> {
    let data = rabject.data.clone();
    AnimWithParams::new(EntityAnim::new(rabject.clone(), data))
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
