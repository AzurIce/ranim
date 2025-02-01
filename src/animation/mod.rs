pub mod composition;
pub mod entity;
pub mod wait;

use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{
    context::WgpuContext,
    items::Entity,
    render::CameraFrame,
    utils::{rate_functions::smooth, Id, RenderResourceStorage},
    Rabject,
};

use composition::Chain;
use entity::EntityAnimation;
#[allow(unused)]
use log::trace;
use wait::Blank;

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

pub trait AnimationFunc<T: Entity> {
    fn eval_alpha(&mut self, target: &mut T, alpha: f32);
}

pub trait Animation {
    fn duration(&self) -> Duration;
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

/// AnimationClip is a timeline structure
pub struct AnimationClip {
    ctx: Arc<WgpuContext>,
    entity_anims: HashMap<Id, Chain>,
    cur_t: f32,
}

impl AnimationClip {
    pub fn new(ctx: Arc<WgpuContext>) -> Self {
        Self {
            ctx,
            entity_anims: HashMap::new(),
            cur_t: 0.0,
        }
    }
}

impl AnimationClip {
    /// Create a rabject with an entity, this will allocate an Id and the render_instance for it
    pub fn insert<T: Entity>(&mut self, entity: T) -> Rabject<T> {
        let rabject = Rabject::new(&self.ctx, entity);
        assert!(!self.entity_anims.contains_key(&rabject.id()));
        // fill up the time before entity is inserted
        self.entity_anims.insert(
            rabject.id(),
            Chain::new(vec![Box::new(Blank(Duration::from_secs_f32(self.cur_t)))]),
        );
        rabject
    }
    pub fn show<T: Entity>(&mut self, rabject: &mut Rabject<T>) {}
    pub fn hide<T: Entity>(&mut self, rabject: &mut Rabject<T>) {}

    /// append an animation to the clip
    pub fn play<T: Entity + 'static>(&mut self, mut anim: EntityAnimation<T>) -> Rabject<T> {
        anim.update_alpha(1.0);
        let result_rabject = anim.rabject().clone();
        let duration = anim.duration();

        self.entity_anims
            .get_mut(&anim.rabject().id())
            .unwrap()
            .append(anim);
        self.cur_t = self.cur_t + duration.as_secs_f32();

        self.entity_anims
            .iter_mut()
            .filter(|(_, entity_anim)| entity_anim.duration().as_secs_f32() < self.cur_t)
            .for_each(|(_, entity_anim)| {
                entity_anim.append(Blank(duration));
            });

        result_rabject
    }
    pub fn play_stacked<T: Entity + 'static, const X: usize>(
        &mut self,
        mut anims: [EntityAnimation<T>; X],
    ) -> [Rabject<T>; X] {
        let result_rabjects = anims.each_mut().map(|anim| {
            anim.update_alpha(1.0);
            anim.rabject().clone()
        });

        let mut max_duration = Duration::from_secs_f32(0.0);
        for anim in anims {
            max_duration = max_duration.max(anim.duration());

            self.entity_anims
                .get_mut(&anim.rabject().id())
                .unwrap()
                .append(anim);
        }
        self.cur_t = self.cur_t + max_duration.as_secs_f32();
        self.entity_anims
            .iter_mut()
            .filter(|(_, entity_anim)| entity_anim.duration().as_secs_f32() < self.cur_t)
            .for_each(|(_, entity_anim)| {
                entity_anim.append(Blank(max_duration));
            });

        result_rabjects
    }
}

impl Animation for AnimationClip {
    fn duration(&self) -> Duration {
        Duration::from_secs_f32(self.cur_t)
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
        for (_, entity_anim) in self.entity_anims.iter() {
            entity_anim.render(
                ctx,
                pipelines,
                encoder,
                uniforms_bind_group,
                multisample_view,
                target_view,
            );
        }
    }
    fn update_alpha(&mut self, alpha: f32) {
        for (_, entity_anim) in self.entity_anims.iter_mut() {
            entity_anim.update_alpha(alpha);
        }
    }
    fn update_clip_info(&self, ctx: &WgpuContext, camera: &CameraFrame) {
        for (_, entity_anim) in self.entity_anims.iter() {
            entity_anim.update_clip_info(ctx, camera);
        }
    }
}
