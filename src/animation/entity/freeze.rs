use crate::{
    animation::Animator,
    items::{Entity, Rabject},
};

use super::{AnimWithParams, EntityAnim};

pub fn freeze<T: Entity + 'static>(rabject: &Rabject<T>) -> AnimWithParams<EntityAnim<T>> {
    let data = rabject.data.clone();
    AnimWithParams::new(EntityAnim::new(rabject.clone(), data))
}

use crate::render::Renderable;

pub struct Blank;

impl Renderable for Blank {
    fn render(
        &self,
        _ctx: &crate::context::WgpuContext,
        _render_instances: &mut crate::render::primitives::RenderInstances,
        _pipelines: &mut crate::utils::RenderResourceStorage,
        _encoder: &mut wgpu::CommandEncoder,
        _uniforms_bind_group: &wgpu::BindGroup,
        _multisample_view: &wgpu::TextureView,
        _target_view: &wgpu::TextureView,
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
