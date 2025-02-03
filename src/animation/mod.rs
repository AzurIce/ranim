pub mod entity;
pub mod wait;

use std::{any::Any, collections::HashMap, sync::Arc, time::Duration};

use crate::{
    context::WgpuContext,
    items::Entity,
    render::{CameraFrame, Renderable},
    utils::{rate_functions::smooth, Id, RenderResourceStorage},
    Rabject,
};

// use composition::Chain;
use entity::{EntityAnimation, EntityAnimator, EntityTimeline};
#[allow(unused)]
use log::trace;
use wait::{blank, wait};

/// An `Animator` is basically an [`Renderable`] which can responds progress alpha change
pub trait Animator: Renderable {
    fn update_alpha(&mut self, alpha: f32);
}

pub trait AnimatorAny: Animator + Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Animator + Any> AnimatorAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// The param of an [`Animation`] or [`EntityAnimation`]
pub struct AnimationParams {
    /// Default: 1.0
    pub duration_secs: f32,
    /// Default: smooth
    pub rate_func: fn(f32) -> f32,
}

impl Default for AnimationParams {
    fn default() -> Self {
        Self {
            duration_secs: 1.0,
            rate_func: smooth,
        }
    }
}

/// An `Animation` is a wrapper of an `Animator` with some params
///
/// The `Animation` itself is also an `Animator` which maps the
/// input alpha with the rate function to inner `Animator`
pub struct Animation {
    animator: Box<dyn Animator>,
    params: AnimationParams,
}

impl Animation {
    pub fn new(animator: impl Animator + 'static) -> Self {
        Self {
            animator: Box::new(animator),
            params: AnimationParams::default(),
        }
    }
    pub fn with_duration(mut self, duration_secs: f32) -> Self {
        self.params.duration_secs = duration_secs;
        self
    }
    pub fn with_rate_func(mut self, rate_func: fn(f32) -> f32) -> Self {
        self.params.rate_func = rate_func;
        self
    }
    pub fn duration_secs(&self) -> f32 {
        self.params.duration_secs
    }
}

impl Renderable for Animation {
    fn update_clip_info(&mut self, ctx: &WgpuContext, camera: &CameraFrame) {
        self.animator.update_clip_info(ctx, camera);
    }
    fn render(
        &mut self,
        ctx: &WgpuContext,
        pipelines: &mut RenderResourceStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
    ) {
        self.animator.render(
            ctx,
            pipelines,
            encoder,
            uniforms_bind_group,
            multisample_view,
            target_view,
        );
    }
}

impl Animator for Animation {
    fn update_alpha(&mut self, alpha: f32) {
        let alpha = (self.params.rate_func)(alpha);
        self.animator.update_alpha(alpha);
    }
}

/// Timeline of all rabjects
///
/// The Timeline itself is also an [`Animator`] which:
/// - update all RabjectTimeline's alpha
/// - render all RabjectTimeline
pub struct Timeline {
    /// Rabject's Id -> (RabjectTimeline's total_duration_secs, RabjectTimeline)
    rabject_timelines: HashMap<Id, (f32, Box<dyn AnimatorAny>)>,
    elapsed_secs: f32,
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

impl Timeline {
    /// Create a rabject with an entity, this will allocate an Id and the render_instance for it
    pub fn insert<T: Entity + 'static>(&mut self, entity: T) -> Rabject<T> {
        let rabject = Rabject::new(entity);
        assert!(!self.rabject_timelines.contains_key(&rabject.id()));
        let blank_anim = blank(rabject.clone()).with_duration(self.elapsed_secs);

        let (_, timeline) = self.rabject_timelines.entry(rabject.id()).or_insert((
            self.elapsed_secs,
            Box::new(EntityTimeline::new(rabject.clone())),
        ));

        // fill up the time before entity is inserted
        let timeline = timeline
            .as_any_mut()
            .downcast_mut::<EntityTimeline<T>>()
            .unwrap();
        timeline.push(blank_anim);
        rabject
    }
    // pub fn show<T: Entity>(&mut self, rabject: &mut Rabject<T>) {}
    // pub fn hide<T: Entity>(&mut self, rabject: &mut Rabject<T>) {}

    pub fn forward(&mut self, duration: Duration) {
        self.elapsed_secs += duration.as_secs_f32();
    }

    /// append an animation to the clip
    pub fn play<T: Entity + 'static>(&mut self, mut anim: EntityAnimation<T>) -> Rabject<T> {
        let (entity_duration, timeline) = self.rabject_timelines.get_mut(&anim.entity_id).unwrap();
        let timeline = timeline
            .as_any_mut()
            .downcast_mut::<EntityTimeline<T>>()
            .unwrap();

        // Fill the gap with wait
        let gapped_duration = self.elapsed_secs - *entity_duration;
        if gapped_duration > 0.0 {
            timeline.push(wait(timeline.rabject.clone()).with_duration(gapped_duration));
            *entity_duration += gapped_duration;
        }

        *entity_duration += anim.params.duration_secs;
        self.elapsed_secs = self.elapsed_secs + anim.params.duration_secs;
        assert_eq!(*entity_duration, self.elapsed_secs);

        timeline.rabject.inner = anim.eval_alpha(1.0);
        timeline.push(anim);
        timeline.rabject.clone()
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
    fn update_clip_info(&mut self, ctx: &WgpuContext, camera: &CameraFrame) {
        for (_, (_, timeline)) in self.rabject_timelines.iter_mut() {
            timeline.update_clip_info(ctx, camera);
        }
    }
    fn render(
        &mut self,
        ctx: &WgpuContext,
        pipelines: &mut RenderResourceStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
    ) {
        for (_, (_, timeline)) in self.rabject_timelines.iter_mut() {
            timeline.render(
                ctx,
                pipelines,
                encoder,
                uniforms_bind_group,
                multisample_view,
                target_view,
            );
        }
    }
}

impl Animator for Timeline {
    fn update_alpha(&mut self, alpha: f32) {
        for (_, (entity_duration_secs, timeline)) in self.rabject_timelines.iter_mut() {
            // println!("alpha: {alpha}, entity_duration: {}, cur_t: {}", entity_duration, self.cur_t);
            let alpha = (alpha * self.elapsed_secs / *entity_duration_secs).clamp(0.0, 1.0);
            timeline.update_alpha(alpha);
        }
    }
}
