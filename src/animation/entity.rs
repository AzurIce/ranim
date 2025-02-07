//! The EntityAnimation is applied to an entity
//!
pub mod composition;
pub mod creation;
pub mod fading;
pub mod freeze;
pub mod interpolate;

use std::rc::Rc;

use freeze::{freeze, Blank};
use itertools::Itertools;

use crate::{
    context::WgpuContext,
    items::{Entity, Rabject},
    render::{primitives::RenderInstances, CameraFrame, RenderTextures, Renderable},
    utils::PipelinesStorage,
};

use super::{Anim, AnimWithParams, Animator, StaticAnim};

pub struct EntityTimeline {
    // pub(super) rabject_id: Id,
    pub(super) cur_freeze_anim: StaticAnim,
    pub(super) is_showing: bool,
    pub(super) cur_anim_idx: Option<usize>,
    pub(super) anims: Vec<Anim>,
    pub(super) end_secs: Vec<f32>,
    pub(super) elapsed_secs: f32,
}

impl EntityTimeline {
    pub fn new<T: Entity + 'static>(rabject: &Rabject<T>) -> Self {
        Self {
            // rabject_id: rabject.id,
            cur_freeze_anim: Rc::new(Box::new(freeze(rabject))),
            cur_anim_idx: None,
            is_showing: true,
            anims: Vec::new(),
            end_secs: Vec::new(),
            elapsed_secs: 0.0,
        }
    }
    fn push<T: Animator + 'static>(&mut self, anim: AnimWithParams<T>) {
        let duration = anim.params.duration_secs;
        self.anims.push(Box::new(anim));

        let end_sec = self.end_secs.last().copied().unwrap_or(0.0) + duration;
        self.end_secs.push(end_sec);
        self.elapsed_secs += duration;
    }

    /// Simply [`Self::append_freeze`] after used [`super::timeline::Timeline::show`],
    /// and [`Self::append_blank`] after used [`super::timeline::Timeline::hide`].
    pub fn forward(&mut self, secs: f32) {
        if self.is_showing {
            self.append_freeze(secs);
        } else {
            self.append_blank(secs);
        }
    }
    /// Append a freeze animation to the timeline
    ///
    /// A freeze animation just keeps the last frame of the previous animation
    pub fn append_freeze(&mut self, secs: f32) {
        self.push(AnimWithParams::new(self.cur_freeze_anim.clone()).with_duration(secs))
    }
    /// Append a blank animation to the timeline
    ///
    /// A blank animation renders nothing
    pub fn append_blank(&mut self, secs: f32) {
        self.push(AnimWithParams::new(Blank).with_duration(secs));
    }
    /// Append an animation to the timeline
    pub fn append_anim<T: Entity + 'static>(
        &mut self,
        mut anim: AnimWithParams<EntityAnim<T>>,
    ) -> Rabject<T> {
        anim.update_alpha(1.0);
        let end_rabject = anim.anim.rabject.clone();

        self.cur_freeze_anim = Rc::new(Box::new(freeze(&end_rabject)));
        self.push(anim);
        end_rabject
    }
}

impl Animator for EntityTimeline {
    fn update_alpha(&mut self, alpha: f32) {
        // TODO: handle no anim
        if self.anims.is_empty() {
            return;
        }
        // trace!("update_alpha: {alpha}, {}", self.elapsed_secs);
        let cur_sec = alpha * self.elapsed_secs;
        let (idx, (anim, end_sec)) = self
            .anims
            .iter_mut()
            .zip(self.end_secs.iter())
            .find_position(|(_, end_sec)| **end_sec >= cur_sec)
            .unwrap();
        // trace!("{cur_sec}[{idx}] {:?}", self.end_secs);
        self.cur_anim_idx = Some(idx);

        let start_sec = if idx > 0 {
            self.end_secs.get(idx - 1).cloned()
        } else {
            None
        }
        .unwrap_or(0.0);
        let alpha = (cur_sec - start_sec) / (end_sec - start_sec);
        anim.update_alpha(alpha);
    }
}

impl Renderable for EntityTimeline {
    fn render(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        pipelines: &mut PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        camera: &CameraFrame,
    ) {
        if let Some(idx) = self.cur_anim_idx {
            self.anims[idx].render(
                ctx,
                render_instances,
                pipelines,
                encoder,
                uniforms_bind_group,
                render_textures,
                camera,
            );
        }
    }
}

/// An animator that animates an entity
pub trait PureEvaluator<T: Entity> {
    fn eval_alpha(&self, alpha: f32) -> T;
}

impl<T: Entity> PureEvaluator<T> for T {
    fn eval_alpha(&self, _alpha: f32) -> T {
        self.clone()
    }
}
impl<T: Entity> PureEvaluator<T> for Rabject<T> {
    fn eval_alpha(&self, _alpha: f32) -> T {
        self.data.clone()
    }
}

/// An animation that is applied to an entity
/// 
/// This type implements [`Animator`] and [`Renderable`]
#[derive(Clone)]
pub struct EntityAnim<T: Entity> {
    rabject: Rabject<T>,
    evaluator: Rc<Box<dyn PureEvaluator<T>>>,
}

impl<T: Entity + 'static> Animator for EntityAnim<T> {
    fn update_alpha(&mut self, alpha: f32) {
        self.rabject.data = self.evaluator.eval_alpha(alpha);
    }
}

impl<T: Entity + 'static> Renderable for EntityAnim<T> {
    fn render(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        pipelines: &mut PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        camera: &CameraFrame,
    ) {
        self.rabject.render(
            ctx,
            render_instances,
            pipelines,
            encoder,
            uniforms_bind_group,
            render_textures,
            camera,
        );
    }
}

impl<T: Entity> EntityAnim<T> {
    pub fn new(rabject: Rabject<T>, func: impl PureEvaluator<T> + 'static) -> Self {
        Self {
            rabject,
            evaluator: Rc::new(Box::new(func)),
        }
    }
    pub fn rabject(&self) -> &Rabject<T> {
        &self.rabject
    }
}
