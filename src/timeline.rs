use crate::{
    animation::{
        freeze::Blank, Anim, AnimScheduler, AnimWithParams, Animator, EntityAnim, StaticAnim,
    },
    context::WgpuContext,
    items::{Entity, Rabject},
    render::{primitives::RenderInstances, CameraFrame, RenderTextures, Renderable},
    utils::{Id, PipelinesStorage},
};
use itertools::Itertools;
use std::{cell::RefCell, rc::Rc};
use std::{collections::HashMap, fmt::Debug, time::Duration};

pub use ranim_macros::timeline;

// MARK: Timeline

/// Timeline of all rabjects
///
/// The Timeline itself is also an [`Animator`] which:
/// - update all RabjectTimeline's alpha
/// - render all RabjectTimeline
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
    /// to apply the animation effect use [`AnimScheduler::apply`]
    pub fn play<'t, T: Entity + 'static>(
        &'t self,
        anim_schedule: AnimScheduler<'_, 't, T, EntityAnim<T>>,
    ) {
        let mut timelines = self.rabject_timelines.borrow_mut();
        let AnimScheduler {
            rabject,
            anim,
            params,
        } = anim_schedule;
        // Fills the gap between the last animation and the current time
        let timeline = timelines.get_mut(&rabject.id).unwrap();

        // Fill the gap with its freeze
        let gapped_duration = *self.elapsed_secs.borrow_mut() - timeline.elapsed_secs;
        if gapped_duration > 0.0 {
            timeline.forward(gapped_duration);
        }

        // Append the animation
        let duration = params.duration_secs;
        *self.elapsed_secs.borrow_mut() += duration;
        timeline.append_anim(AnimWithParams { anim, params });

        // Forword other timelines
        for (_id, timeline) in timelines.iter_mut() {
            if timeline.elapsed_secs < self.elapsed_secs() {
                timeline.forward(self.elapsed_secs() - timeline.elapsed_secs);
            }
        }
    }
    // pub fn play_stacked<T: Entity + 'static, const X: usize>(
    //     &mut self,
    //     mut anims: [EntityAnimation<T>; X],
    // ) -> [Rabject<T>; X] {
    //     let result_rabjects = anims.each_mut().map(|anim| {
    //         let timeline = self.entity_timelines.get_mut(&anim.entity_id).unwrap();
    //         let timeline = timeline
    //             .as_any_mut()
    //             .downcast_mut::<EntityTimeline<T>>()
    //             .unwrap();

    //         timeline.rabject.inner = anim.eval_alpha(1.0);
    //         timeline.rabject.clone()
    //     });

    //     let mut max_duration = anims.iter().map(|anim| anim.params.duration).max().unwrap();

    //     for anim in anims {
    //         let timeline = self
    //             .entity_timelines
    //             .get_mut(&anim.entity_id)
    //             .map(|timeline| {
    //                 timeline
    //                     .as_any_mut()
    //                     .downcast_mut::<EntityTimeline<T>>()
    //                     .unwrap()
    //             })
    //             .unwrap();
    //         let rabject = timeline.rabject.clone();
    //         let duration = max_duration - anim.params.duration;
    //         timeline.push(anim);
    //         timeline.push(wait(rabject).with_duration(duration));
    //     }
    //     self.cur_t = self.cur_t + max_duration.as_secs_f32();
    //     self.entity_timelines
    //         .iter_mut()
    //         .filter(|(id, _)| {
    //             result_rabjects
    //                 .iter()
    //                 .all(|result_rabject| **id != result_rabject.id)
    //         })
    //         .map(|(_, timeline)| {
    //             timeline
    //                 .as_any_mut()
    //                 .downcast_mut::<EntityTimeline<T>>()
    //                 .unwrap()
    //         })
    //         .for_each(|timeline| {
    //             let rabject = timeline.rabject.clone();
    //             timeline.push(blank(rabject).with_duration(max_duration));
    //         });

    //     result_rabjects
    // }
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
        camera: &CameraFrame,
    ) {
        for (_, timeline) in self.rabject_timelines.borrow().iter() {
            timeline.render(
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

impl Animator for Timeline {
    fn update_alpha(&mut self, alpha: f32) {
        for (_, timeline) in self.rabject_timelines.borrow_mut().iter_mut() {
            timeline.update_alpha(alpha);
        }
    }
}

// MARK: EntityTimeline

#[derive(Clone)]
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
    pub fn new<'a, T: Entity + 'static>(rabject: &'a Rabject<'a, T>) -> Self {
        let freeze_anim: Box<dyn Renderable> = Box::new(EntityAnim::new(
            rabject.id,
            rabject.data.clone(),
            rabject.data.clone(),
        ));
        Self {
            // rabject_id: rabject.id,
            cur_freeze_anim: Rc::new(freeze_anim as Box<dyn Renderable>),
            cur_anim_idx: None,
            is_showing: true,
            anims: Vec::new(),
            end_secs: Vec::new(),
            elapsed_secs: 0.0,
        }
    }
    fn push<T: Animator + 'static>(&mut self, anim: AnimWithParams<T>) {
        let duration = anim.params.duration_secs;
        self.anims.push(Rc::new(RefCell::new(Box::new(anim))));

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
    pub fn append_anim<T: Entity + 'static>(&mut self, mut anim: AnimWithParams<EntityAnim<T>>) {
        anim.update_alpha(1.0);
        self.cur_freeze_anim = Rc::new(Box::new(anim.anim.get_end_freeze_anim()));
        self.push(anim);
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
        anim.borrow_mut().update_alpha(alpha);
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
            self.anims[idx].borrow().render(
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
