use std::time::Duration;

use crate::{items::Entity, Rabject};

use super::{entity::EntityAnimation, Animation, AnimationFunc};

pub fn wait<T: Entity>(rabject: Rabject<T>) -> EntityAnimation<T> {
    EntityAnimation::new(rabject, Wait)
}

pub struct Wait;

impl<T: Entity> AnimationFunc<T> for Wait {
    fn eval_alpha(&mut self, _target: &mut T, _alpha: f32) {}
}

pub struct Blank(pub Duration);

impl Animation for Blank {
    fn duration(&self) -> std::time::Duration {
        self.0
    }
    fn render(
        &self,
        _ctx: &crate::context::WgpuContext,
        _pipelines: &mut crate::utils::RenderResourceStorage,
        _encoder: &mut wgpu::CommandEncoder,
        _uniforms_bind_group: &wgpu::BindGroup,
        _multisample_view: &wgpu::TextureView,
        _target_view: &wgpu::TextureView,
    ) {
        // do nothing
    }
    fn update_alpha(&mut self, _alpha: f32) {
        // do nothing
    }
    fn update_clip_info(
        &self,
        _ctx: &crate::context::WgpuContext,
        _camera: &crate::render::CameraFrame,
    ) {
        // do nothing
    }
}
