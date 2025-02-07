pub mod entity;

use std::{collections::HashMap, fmt::Debug, time::Duration};

use crate::{
    context::WgpuContext,
    items::{Entity, Rabject},
    render::{primitives::RenderInstances, CameraFrame, Renderable},
    utils::{rate_functions::smooth, Id, RenderResourceStorage},
};

use entity::{AnimWithParams, EntityAnim, EntityTimeline};
#[allow(unused)]
use log::trace;

/// An `Animator` is basically an [`Renderable`] which can responds progress alpha change
pub trait Animator: Renderable {
    fn update_alpha(&mut self, alpha: f32);
}

/// The param of an [`Animation`] or [`EntityAnimation`]
#[derive(Debug, Clone)]
pub struct AnimParams {
    /// Default: 1.0
    pub duration_secs: f32,
    /// Default: smooth
    pub rate_func: fn(f32) -> f32,
}

impl Default for AnimParams {
    fn default() -> Self {
        Self {
            duration_secs: 1.0,
            rate_func: smooth,
        }
    }
}

impl AnimParams {
    pub fn with_duration(mut self, duration_secs: f32) -> Self {
        self.duration_secs = duration_secs;
        self
    }
    pub fn with_rate_func(mut self, rate_func: fn(f32) -> f32) -> Self {
        self.rate_func = rate_func;
        self
    }
}

/// Timeline of all rabjects
///
/// The Timeline itself is also an [`Animator`] which:
/// - update all RabjectTimeline's alpha
/// - render all RabjectTimeline
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
        Self {
            rabject_timelines: HashMap::new(),
            elapsed_secs: 0.0,
        }
    }
    pub fn elapsed_secs(&self) -> f32 {
        self.elapsed_secs
    }
}

pub trait TimelineEntity<T: Entity> {
    fn to_rabject(self) -> Rabject<T>;
}

impl<T: Entity + 'static> TimelineEntity<T> for T {
    fn to_rabject(self) -> Rabject<T> {
        Rabject::new(self)
    }
}

impl<T: Entity + 'static> TimelineEntity<T> for Rabject<T> {
    fn to_rabject(self) -> Rabject<T> {
        self
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
        pipelines: &mut RenderResourceStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
        camera: &CameraFrame,
    ) {
        for (_, timeline) in self.rabject_timelines.iter() {
            timeline.render(
                ctx,
                render_instances,
                pipelines,
                encoder,
                uniforms_bind_group,
                multisample_view,
                target_view,
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
