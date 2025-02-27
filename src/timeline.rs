use crate::{
    animation::{
        blank::Blank, AnimSchedule, AnimWithParams, Animation, EntityAnim, Freezable,
        StaticEntityAnim,
    },
    context::WgpuContext,
    items::{Entity, Rabject},
    render::{
        primitives::RenderInstances, DynamicRenderable, RenderTextures, Renderable,
        StaticRenderable,
    },
    utils::{Id, PipelinesStorage},
};
use std::{cell::RefCell, rc::Rc};
use std::{collections::HashMap, fmt::Debug, time::Duration};

use itertools::Itertools;
pub use ranim_macros::timeline;

// MARK: Timeline

/// Timeline of all rabjects
///
/// Timeline has the interior mutability, and its [`Rabject`]s has the reference to it with the same lifetime.
#[derive(Default, Clone)]
pub struct Timeline {
    /// Rabject's Id -> EntityTimeline
    rabject_timelines: RefCell<HashMap<Id, EntityTimeline>>,
    elapsed_secs: RefCell<f32>,
}

impl Debug for Timeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Timeline {:?}:\n",
            Duration::from_secs_f32(*self.elapsed_secs.borrow())
        ))?;
        for (id, timeline) in self.rabject_timelines.borrow().iter() {
            f.write_fmt(format_args!(
                "  EntityTimeline<{:?}>: {:?}\n",
                id,
                Duration::from_secs_f32(timeline.elapsed_secs)
            ))?;
        }
        Ok(())
    }
}

impl Timeline {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn elapsed_secs(&self) -> f32 {
        *self.elapsed_secs.borrow()
    }
}

impl Timeline {
    pub fn insert<T: Entity + 'static>(&self, item: T) -> Rabject<'_, T> {
        let mut timelines = self.rabject_timelines.borrow_mut();
        let rabject = Rabject::new(self, item);
        let timeline = timelines
            .entry(rabject.id)
            .or_insert(EntityTimeline::new(&rabject));
        if self.elapsed_secs() != 0.0 {
            timeline.append_blank(self.elapsed_secs());
        }
        rabject
    }
    pub fn update<T: Entity + 'static>(&self, rabject: &Rabject<T>) {
        let mut timelines = self.rabject_timelines.borrow_mut();
        let timeline = timelines.get_mut(&rabject.id).unwrap();
        timeline.update(rabject);
    }

    pub fn show<T: Entity>(&self, rabject: &Rabject<T>) {
        self.rabject_timelines
            .borrow_mut()
            .get_mut(&rabject.id)
            .unwrap()
            .is_showing = true;
    }
    pub fn hide<T: Entity>(&self, rabject: &Rabject<T>) {
        self.rabject_timelines
            .borrow_mut()
            .get_mut(&rabject.id)
            .unwrap()
            .is_showing = false;
    }

    pub fn forward(&self, secs: f32) {
        self.rabject_timelines
            .borrow_mut()
            .iter_mut()
            .for_each(|(_id, timeline)| {
                timeline.forward(secs);
            });
        *self.elapsed_secs.borrow_mut() += secs;
    }

    /// Push an animation into the timeline
    ///
    /// Note that this won't apply the animation effect to rabject's data,
    /// to apply the animation effect use [`AnimSchedule::apply`]
    pub fn play<'t, T: Entity + 'static>(
        &'t self,
        anim_schedule: AnimSchedule<'_, 't, T, EntityAnim<T>>,
    ) {
        let mut timelines = self.rabject_timelines.borrow_mut();
        let AnimSchedule { rabject, anim } = anim_schedule;
        // Fills the gap between the last animation and the current time
        let timeline = timelines.get_mut(&rabject.id).unwrap();

        // Fill the gap with its freeze
        let gapped_duration = *self.elapsed_secs.borrow_mut() - timeline.elapsed_secs;
        if gapped_duration > 0.0 {
            timeline.forward(gapped_duration);
        }

        // Append the animation
        let duration = anim.params.duration_secs;
        *self.elapsed_secs.borrow_mut() += duration;
        timeline.append_anim(anim);

        // Forword other timelines
        for (_id, timeline) in timelines.iter_mut() {
            if timeline.elapsed_secs < self.elapsed_secs() {
                timeline.forward(self.elapsed_secs() - timeline.elapsed_secs);
            }
        }
    }
}

impl DynamicRenderable for Timeline {
    fn prepare_alpha(
        &mut self,
        alpha: f32,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
    ) {
        for (_id, timeline) in self.rabject_timelines.borrow_mut().iter_mut() {
            timeline.prepare_alpha(alpha, ctx, render_instances);
        }
    }
}

impl Renderable for Timeline {
    fn render(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        pipelines: &mut PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
    ) {
        for (_id, timeline) in self.rabject_timelines.borrow().iter() {
            timeline.render(
                ctx,
                render_instances,
                pipelines,
                encoder,
                uniforms_bind_group,
                render_textures,
            );
        }
    }
}

// MARK: EntityTimeline

#[derive(Clone)]
pub struct EntityTimeline {
    // pub(super) rabject_id: Id,
    pub(super) cur_freeze_anim: Rc<Box<dyn StaticRenderable>>,
    pub(super) is_showing: bool,
    pub(super) last_anim_idx: Option<usize>,
    pub(super) cur_anim_idx: Option<usize>,
    pub(super) anims: Vec<Animation>,
    pub(super) end_secs: Vec<f32>,
    pub(super) elapsed_secs: f32,
}

impl EntityTimeline {
    pub fn update<T: Entity + 'static>(&mut self, rabject: &Rabject<T>) {
        let freeze_anim: Box<dyn StaticRenderable> =
            Box::new(StaticEntityAnim::new(rabject.id, rabject.data.clone()));
        self.cur_freeze_anim = Rc::new(freeze_anim);
    }
    pub fn new<'a, T: Entity + 'static>(rabject: &'a Rabject<'a, T>) -> Self {
        let freeze_anim: Box<dyn StaticRenderable> =
            Box::new(StaticEntityAnim::new(rabject.id, rabject.data.clone()));
        Self {
            // rabject_id: rabject.id,
            cur_freeze_anim: Rc::new(freeze_anim),
            last_anim_idx: None,
            cur_anim_idx: None,
            is_showing: true,
            anims: Vec::new(),
            end_secs: Vec::new(),
            elapsed_secs: 0.0,
        }
    }
    fn push<A: Into<Animation>>(&mut self, anim: AnimWithParams<A>) {
        let duration = anim.params.duration_secs;
        self.anims.push(anim.into());

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
        self.push(
            AnimWithParams::new(Animation::Static(self.cur_freeze_anim.clone()))
                .with_duration(secs),
        )
    }

    /// Append a blank animation to the timeline
    ///
    /// A blank animation renders nothing
    pub fn append_blank(&mut self, secs: f32) {
        self.push(
            AnimWithParams::new(Animation::Static(Rc::new(Box::new(Blank)))).with_duration(secs),
        );
    }

    /// Append an animation to the timeline
    pub fn append_anim<T: Entity + 'static>(&mut self, anim: AnimWithParams<EntityAnim<T>>) {
        self.cur_freeze_anim = Rc::new(Box::new(anim.inner.get_end_freeze_anim()));
        self.push(anim);
    }
}

impl DynamicRenderable for EntityTimeline {
    fn prepare_alpha(
        &mut self,
        alpha: f32,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
    ) {
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
        // trace!("{cur_sec[{idx}] {:?}", self.end_secs);
        self.last_anim_idx = self.cur_anim_idx;
        self.cur_anim_idx = Some(idx);
        let start_sec = if idx > 0 {
            self.end_secs.get(idx - 1).cloned()
        } else {
            None
        }
        .unwrap_or(0.0);
        let alpha = (cur_sec - start_sec) / (end_sec - start_sec);
        match anim {
            Animation::Static(anim) => {
                if self.last_anim_idx != self.cur_anim_idx {
                    anim.prepare(ctx, render_instances);
                }
            }
            Animation::Dynamic(anim) => {
                anim.prepare_alpha(alpha, ctx, render_instances);
            }
        }
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
    ) {
        if let Some(idx) = self.cur_anim_idx {
            self.anims[idx].render(
                ctx,
                render_instances,
                pipelines,
                encoder,
                uniforms_bind_group,
                render_textures,
            );
        }
    }
}
