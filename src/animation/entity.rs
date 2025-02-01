//! The EntityAnimation is applied to an entity
//!
pub mod creation;
pub mod fading;
pub mod interpolate;

use std::{cell::RefCell, rc::Rc, time::Duration};

use crate::{
    context::WgpuContext,
    items::Entity,
    render::{primitives::Primitive, CameraFrame},
    utils::RenderResourceStorage,
    Rabject,
};

use super::{Animation, AnimationFunc, AnimationParams};

pub struct EntityAnimation<T: Entity> {
    animating_rabject: Rabject<T>,

    param: AnimationParams,
    func: Box<dyn AnimationFunc<T>>,
}

impl<T: Entity> EntityAnimation<T> {
    pub fn rabject(&self) -> &Rabject<T> {
        &self.animating_rabject
    }
    pub fn new(rabject: Rabject<T>, func: impl AnimationFunc<T> + 'static) -> Self {
        Self {
            animating_rabject: rabject.clone(),
            param: Default::default(),
            func: Box::new(func),
        }
    }
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.param.duration = duration;
        self
    }
    pub fn with_rate_func(mut self, rate_func: fn(f32) -> f32) -> Self {
        self.param.rate_func = rate_func;
        self
    }
}

impl<T: Entity> Animation for EntityAnimation<T> {
    fn duration(&self) -> Duration {
        self.param.duration
    }
    fn update_alpha(&mut self, alpha: f32) {
        let alpha = (self.param.rate_func)(alpha);
        self.func
            .eval_alpha(&mut self.animating_rabject.inner, alpha);
    }
    fn update_clip_info(&self, ctx: &WgpuContext, camera: &CameraFrame) {
        self.animating_rabject
            .render_instance
            .borrow_mut()
            .update_clip_info(ctx, camera);
    }
    fn render(
        &self,
        ctx: &WgpuContext,
        pipelines: &mut RenderResourceStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
    ) {
        let mut render_instance = self.animating_rabject.render_instance.borrow_mut();
        render_instance.update(ctx, &self.animating_rabject.inner);
        render_instance.encode_render_command(
            ctx,
            pipelines,
            encoder,
            uniforms_bind_group,
            multisample_view,
            target_view,
        );
    }
}
