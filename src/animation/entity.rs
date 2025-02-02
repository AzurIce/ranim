//! The EntityAnimation is applied to an entity
//!
pub mod creation;
pub mod fading;
pub mod interpolate;

use std::{
    any::Any,
    ops::{Deref, DerefMut, RangeInclusive},
    time::Duration,
};

use itertools::Itertools;

use crate::{
    context::WgpuContext,
    items::Entity,
    render::{primitives::Primitive, CameraFrame},
    utils::{Id, RenderResourceStorage},
    Rabject,
};

use super::{AnimationParams, Animator};

pub struct EntityTimeline<T: Entity> {
    pub(crate) rabject: Rabject<T>,
    anims: Vec<EntityAnimation<T>>,
    end_sec: Vec<f32>,
    total_sec: f32,
}

impl<T: Entity> EntityTimeline<T> {
    pub fn new(rabject: Rabject<T>) -> Self {
        Self {
            rabject,
            anims: Vec::new(),
            end_sec: Vec::new(),
            total_sec: 0.0,
        }
    }
    /// Push an animation to the timeline
    pub fn push(&mut self, anim: EntityAnimation<T>) {
        let duration = anim.params.duration.as_secs_f32();
        self.anims.push(anim);

        let end_sec = self.end_sec.last().copied().unwrap_or(0.0) + duration;
        self.end_sec.push(end_sec);
        self.total_sec += duration;
    }
}

impl<T: Entity> Animator for EntityTimeline<T> {
    fn update_alpha(&mut self, alpha: f32) {
        // println!("update_alpha: {alpha}, {}", self.total_sec);
        // println!("{:?}", self.end_sec);
        let sec = alpha * self.total_sec;
        let (anim, end_sec) = self
            .anims
            .iter_mut()
            .zip(self.end_sec.iter())
            .find(|(_, end_sec)| **end_sec >= sec)
            .unwrap();
        let start_sec = end_sec - anim.params.duration.as_secs_f32();
        let alpha = (sec - start_sec) / (end_sec - start_sec);
        self.rabject.inner = anim.eval_alpha(alpha);
    }
    fn update_clip_info(&self, ctx: &WgpuContext, camera: &CameraFrame) {
        self.rabject
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
        let mut render_instance = self.rabject.render_instance.borrow_mut();
        render_instance.update(ctx, &self.rabject.inner);
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

pub trait EntityAnimator<T: Entity> {
    fn eval_alpha(&mut self, alpha: f32) -> T;
}

pub struct EntityAnimation<T: Entity> {
    pub(crate) entity_id: Id,
    animator: Box<dyn EntityAnimator<T>>,
    pub(crate) params: AnimationParams,
}

impl<T: Entity> EntityAnimation<T> {
    pub fn new(entity_id: Id, func: impl EntityAnimator<T> + 'static) -> Self {
        Self {
            entity_id,
            animator: Box::new(func),
            params: AnimationParams::default(),
        }
    }
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.params.duration = duration;
        self
    }
    pub fn with_rate_func(mut self, rate_func: fn(f32) -> f32) -> Self {
        self.params.rate_func = rate_func;
        self
    }
}

impl<T: Entity> EntityAnimator<T> for EntityAnimation<T> {
    fn eval_alpha(&mut self, alpha: f32) -> T {
        let alpha = (self.params.rate_func)(alpha);
        self.animator.eval_alpha(alpha)
    }
}

// impl<T: Entity> Animator for EntityAnimation<T> {
//     fn update_alpha(&mut self, alpha: f32) {
//         self.animating_rabject.inner = self.animator.eval_alpha(alpha);
//     }
//     fn update_clip_info(&self, ctx: &WgpuContext, camera: &CameraFrame) {
//         self.animating_rabject
//             .render_instance
//             .borrow_mut()
//             .update_clip_info(ctx, camera);
//     }
//     fn render(
//         &self,
//         ctx: &WgpuContext,
//         pipelines: &mut RenderResourceStorage,
//         encoder: &mut wgpu::CommandEncoder,
//         uniforms_bind_group: &wgpu::BindGroup,
//         multisample_view: &wgpu::TextureView,
//         target_view: &wgpu::TextureView,
//     ) {
//         let mut render_instance = self.animating_rabject.render_instance.borrow_mut();
//         render_instance.update(ctx, &self.animating_rabject.inner);
//         render_instance.encode_render_command(
//             ctx,
//             pipelines,
//             encoder,
//             uniforms_bind_group,
//             multisample_view,
//             target_view,
//         );
//     }
// }
