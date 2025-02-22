use crate::{
    animation::{
        freeze::{freeze, Blank},
        Anim, AnimWithParams, Animator, EntityAnim, StaticAnim,
    },
    context::WgpuContext,
    items::{Entity, Rabject},
    render::{primitives::RenderInstances, CameraFrame, RenderTextures, Renderable},
    utils::{Id, PipelinesStorage},
};
use itertools::Itertools;
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, fmt::Debug, time::Duration};

// MARK: Timeline

/// Timeline of all rabjects
///
/// The Timeline itself is also an [`Animator`] which:
/// - update all RabjectTimeline's alpha
/// - render all RabjectTimeline
#[derive(Default, Clone)]
pub struct Timeline {
    /// Rabject's Id -> EntityTimeline
    rabject_timelines: HashMap<Id, EntityTimeline>,
    elapsed_secs: f32,
}

impl Debug for Timeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Timeline {:?}:\n",
            Duration::from_secs_f32(self.elapsed_secs)
        ))?;
        for (id, timeline) in self.rabject_timelines.iter() {
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
        self.elapsed_secs
    }
}

impl Timeline {
    fn get_or_init_entity_timeline<T: Entity + 'static>(
        &mut self,
        rabject: &Rabject<T>,
    ) -> &mut EntityTimeline {
        self.rabject_timelines
            .entry(rabject.id)
            .or_insert(EntityTimeline::new(rabject))
    }
    pub fn show<T: Entity + 'static>(&mut self, rabject: &Rabject<T>) {
        self.get_or_init_entity_timeline(rabject).is_showing = true;
    }
    pub fn hide<T: Entity + 'static>(&mut self, rabject: &Rabject<T>) {
        self.get_or_init_entity_timeline(rabject).is_showing = false;
    }

    pub fn forward(&mut self, secs: f32) {
        self.rabject_timelines
            .iter_mut()
            .for_each(|(_id, timeline)| {
                timeline.forward(secs);
            });
        self.elapsed_secs += secs;
    }

    /// append an animation to the clip
    pub fn play<T: Entity + 'static>(&mut self, anim: AnimWithParams<EntityAnim<T>>) -> Rabject<T> {
        let (duration, end_rabject) = {
            // Fills the gap between the last animation and the current time
            let rabject = anim.anim.rabject();
            let timeline = if let Some(timeline) = self.rabject_timelines.get_mut(&rabject.id) {
                let gapped_duration = self.elapsed_secs - timeline.elapsed_secs;
                if gapped_duration > 0.0 {
                    timeline.forward(gapped_duration);
                }
                timeline
            } else {
                let elapsed_secs = self.elapsed_secs;
                let timeline = self.get_or_init_entity_timeline(rabject);
                timeline.append_blank(elapsed_secs);
                timeline
            };

            // Append the animation
            let duration = anim.params.duration_secs;
            let rabject = timeline.append_anim(anim);
            (duration, rabject)
        };
        self.elapsed_secs += duration;

        // Forword other timelines
        for (_id, timeline) in self.rabject_timelines.iter_mut() {
            if timeline.elapsed_secs < self.elapsed_secs {
                timeline.forward(self.elapsed_secs - timeline.elapsed_secs);
            }
        }
        end_rabject
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
        for (_, timeline) in self.rabject_timelines.iter() {
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
        for (_, timeline) in self.rabject_timelines.iter_mut() {
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
    pub fn new<T: Entity + 'static>(rabject: &Rabject<T>) -> Self {
        Self {
            // rabject_id: rabject.id,
            cur_freeze_anim: Arc::new(Box::new(freeze(rabject))),
            cur_anim_idx: None,
            is_showing: true,
            anims: Vec::new(),
            end_secs: Vec::new(),
            elapsed_secs: 0.0,
        }
    }
    fn push<T: Animator + 'static>(&mut self, anim: AnimWithParams<T>) {
        let duration = anim.params.duration_secs;
        self.anims.push(Arc::new(Mutex::new(Box::new(anim))));

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

        self.cur_freeze_anim = Arc::new(Box::new(freeze(&end_rabject)));
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
        anim.lock().unwrap().update_alpha(alpha);
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
            self.anims[idx].lock().unwrap().render(
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
