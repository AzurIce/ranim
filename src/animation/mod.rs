pub mod composition;
pub mod entity;
pub mod wait;

use std::{
    any::{Any, TypeId},
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::{self, Duration},
};

use crate::{
    context::WgpuContext,
    items::Entity,
    prelude::Empty,
    render::CameraFrame,
    utils::{rate_functions::smooth, Id, RenderResourceStorage},
    Rabject,
};

// use composition::Chain;
use entity::{EntityAnimation, EntityAnimator, EntityTimeline};
#[allow(unused)]
use log::trace;
use wait::{blank, wait, Blank};

pub struct AnimationParams {
    pub duration: Duration,
    pub rate_func: fn(f32) -> f32,
}

impl Default for AnimationParams {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs_f32(1.0),
            rate_func: smooth,
        }
    }
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

pub trait Animator {
    fn update_alpha(&mut self, alpha: f32);
    fn update_clip_info(&self, ctx: &WgpuContext, camera: &CameraFrame);
    fn render(
        &self,
        ctx: &WgpuContext,
        pipelines: &mut RenderResourceStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
    );
}

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
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.params.duration = duration;
        self
    }
    pub fn with_rate_func(mut self, rate_func: fn(f32) -> f32) -> Self {
        self.params.rate_func = rate_func;
        self
    }
    pub fn duration(&self) -> Duration {
        self.params.duration
    }
}

impl Animator for Animation {
    fn update_alpha(&mut self, alpha: f32) {
        let alpha = (self.params.rate_func)(alpha);
        self.animator.update_alpha(alpha);
    }
    fn update_clip_info(&self, ctx: &WgpuContext, camera: &CameraFrame) {
        self.animator.update_clip_info(ctx, camera);
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

/// Timeline of animations
pub struct Timeline {
    ctx: Arc<WgpuContext>,
    entity_timelines: HashMap<Id, (f32, Box<dyn AnimatorAny>)>,
    cur_t: f32,
}

impl Timeline {
    pub fn new(ctx: Arc<WgpuContext>) -> Self {
        Self {
            ctx,
            entity_timelines: HashMap::new(),
            cur_t: 0.0,
        }
    }
    pub fn cur_t(&self) -> f32 {
        self.cur_t
    }
}

impl Timeline {
    /// Create a rabject with an entity, this will allocate an Id and the render_instance for it
    pub fn insert<T: Entity + 'static>(&mut self, entity: T) -> Rabject<T> {
        let rabject = Rabject::new(&self.ctx, entity);
        assert!(!self.entity_timelines.contains_key(&rabject.id()));
        let blank_anim = blank(rabject.clone()).with_duration(Duration::from_secs_f32(self.cur_t));

        let (_, timeline) = self
            .entity_timelines
            .entry(rabject.id())
            .or_insert((self.cur_t, Box::new(EntityTimeline::new(rabject.clone()))));

        // fill up the time before entity is inserted
        let timeline = timeline
            .as_any_mut()
            .downcast_mut::<EntityTimeline<T>>()
            .unwrap();
        timeline.push(blank_anim);
        rabject
    }
    pub fn show<T: Entity>(&mut self, rabject: &mut Rabject<T>) {}
    pub fn hide<T: Entity>(&mut self, rabject: &mut Rabject<T>) {}

    pub fn forward(&mut self, duration: Duration) {
        self.cur_t += duration.as_secs_f32();
    }

    /// append an animation to the clip
    pub fn play<T: Entity + 'static>(&mut self, mut anim: EntityAnimation<T>) -> Rabject<T> {
        let (entity_duration, timeline) = self.entity_timelines.get_mut(&anim.entity_id).unwrap();
        let timeline = timeline
            .as_any_mut()
            .downcast_mut::<EntityTimeline<T>>()
            .unwrap();

        // Fill the gap with wait
        let gapped_duration = self.cur_t - *entity_duration;
        if gapped_duration > 0.0 {
            timeline.push(
                wait(timeline.rabject.clone())
                    .with_duration(Duration::from_secs_f32(gapped_duration)),
            );
            *entity_duration += gapped_duration;
        }

        let duration = anim.params.duration.as_secs_f32();
        *entity_duration += duration;
        self.cur_t = self.cur_t + duration;
        assert_eq!(*entity_duration, self.cur_t);

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

impl Animator for Timeline {
    fn update_alpha(&mut self, alpha: f32) {
        for (_, (entity_duration, timeline)) in self.entity_timelines.iter_mut() {
            // println!("alpha: {alpha}, entity_duration: {}, cur_t: {}", entity_duration, self.cur_t);
            let alpha = (alpha * self.cur_t / *entity_duration).clamp(0.0, 1.0);
            timeline.update_alpha(alpha);
        }
    }
    fn update_clip_info(&self, ctx: &WgpuContext, camera: &CameraFrame) {
        for (_, (_, timeline)) in self.entity_timelines.iter() {
            timeline.update_clip_info(ctx, camera);
        }
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
        for (_, (_, timeline)) in self.entity_timelines.iter() {
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

// pub struct BuiltTimeline {
//     ctx: Arc<WgpuContext>,
//     entity_timelines: Vec<(Id, Box<dyn Animator>)>,
//     total_duration_sec: f32,
// }

// impl BuiltTimeline {
//     pub fn build_from(timeline: Timeline) -> Self {
//         let total_duration_sec = timeline.cur_t;
//         let entity_timelines = timeline
//             .entity_timelines
//             .into_iter()
//             .map(|(id, timeline)| (id, timeline.build(total_duration_sec)))
//             .collect::<Vec<_>>();
//         Self {
//             ctx: timeline.ctx,
//             entity_timelines,
//             total_duration_sec,
//         }
//     }
// }

// impl Animator for BuiltTimeline {
//     fn render(
//         &self,
//         ctx: &WgpuContext,
//         pipelines: &mut RenderResourceStorage,
//         encoder: &mut wgpu::CommandEncoder,
//         uniforms_bind_group: &wgpu::BindGroup,
//         multisample_view: &wgpu::TextureView,
//         target_view: &wgpu::TextureView,
//     ) {
//         for (_, timeline) in self.entity_timelines.iter() {
//             timeline.render(
//                 ctx,
//                 pipelines,
//                 encoder,
//                 uniforms_bind_group,
//                 multisample_view,
//                 target_view,
//             );
//         }
//     }
//     fn update_alpha(&mut self, alpha: f32) {
//         for (_, timeline) in self.entity_timelines.iter_mut() {
//             timeline.update_alpha(alpha);
//         }
//     }
//     fn update_clip_info(&self, ctx: &WgpuContext, camera: &CameraFrame) {
//         for (_, timeline) in self.entity_timelines.iter() {
//             timeline.update_clip_info(ctx, camera);
//         }
//     }
// }
