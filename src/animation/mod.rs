pub mod fading;
pub mod transform;

use std::{any::Any, ops::Deref, time};

use log::trace;

use crate::{
    rabject::{Rabject, RabjectWithId},
    scene::Scene,
    utils::rate_functions::smooth,
    RanimContext,
};

pub struct AnimationConfig {
    pub run_time: time::Duration,
    pub rate_func: Box<dyn Fn(f32) -> f32>,
    pub remove: bool,
}

impl Default for AnimationConfig {
    /// Default animation config
    /// - run_time: 1.0s
    /// - rate_func: linear
    /// - remove: false
    fn default() -> Self {
        Self {
            run_time: time::Duration::from_secs_f32(1.0),
            rate_func: Box::new(smooth),
            remove: false,
        }
    }
}

impl AnimationConfig {
    pub fn run_time(mut self, run_time: time::Duration) -> Self {
        self.run_time = run_time;
        self
    }

    pub fn rate_func(mut self, rate_func: Box<dyn Fn(f32) -> f32>) -> Self {
        self.rate_func = rate_func;
        self
    }

    pub fn remove(mut self) -> Self {
        self.remove = true;
        self
    }

    pub fn calc_frames(&self, fps: f32) -> usize {
        (self.run_time.as_secs_f32() * fps).ceil() as usize
    }
}

pub trait AnimationFunc<R: Rabject> {
    fn prepare(&mut self, rabject: &mut RabjectWithId<R>, scene: &mut Scene) {}
    #[allow(unused)]
    fn pre_anim(&mut self, rabject: &mut RabjectWithId<R>) {}

    fn interpolate(&mut self, rabject: &mut RabjectWithId<R>, alpha: f32);

    #[allow(unused)]
    fn post_anim(&mut self, rabject: &mut RabjectWithId<R>) {}

    fn cleanup(&mut self, rabject: &mut RabjectWithId<R>, scene: &mut Scene) {}
}

/// A struct representing an animation
///
/// The creation of an animation takes the ownership of the mobject to be animated (which is called "the animated mobject"), and
/// during the animation, this mobject's properties will be modified, but keeps the same id.
///
/// An [`Animation`] doesn't plays itself, it just describe what an animation like.
/// To play an animation, should use [`crate::scene::Scene`]'s [`crate::scene::Scene::play`] method.
///
/// The scene will use [`Animation::func`]'s [`AnimationFunc::interpolate`] method to modify the mobject,
/// and then use [`crate::scene::Scene::add_mobject`] to update the mobject.
///
/// When the animation is done, the scene will return an [`Option<Mobject>`] according to
/// [`AnimationConfig::remove`].
/// If `remove` is `true`, the scene will remove the mobject from the scene and return `None`.
/// Otherwise, the scene will return the modified mobject and keep it in the scene.
pub struct Animation<R: Rabject> {
    /// The mobject to be animated, will take the ownership of it, and return by scene's [`crate::scene::Scene::play`] method
    pub rabject: RabjectWithId<R>,
    pub func: Box<dyn AnimationFunc<R>>,
    pub config: AnimationConfig,
}

impl<R: Rabject> Animation<R> {
    pub fn new(
        rabject: RabjectWithId<R>,
        func: impl AnimationFunc<R> + 'static,
        config: AnimationConfig,
    ) -> Self {
        Self {
            rabject,
            func: Box::new(func),
            config,
        }
    }

    pub fn play(mut self, ctx: &mut RanimContext, scene: &mut Scene) -> Option<RabjectWithId<R>> {
        self.func.prepare(&mut self.rabject, scene);
        self.func.pre_anim(&mut self.rabject);

        let frames = self.config.calc_frames(scene.camera.fps as f32);

        let dt = self.config.run_time.as_secs_f32() / (frames - 1) as f32;
        for t in (0..frames).map(|x| x as f32 * dt) {
            // TODO: implement mobject's updaters
            // animation.update_mobjects(dt);
            let alpha = t / self.config.run_time.as_secs_f32();
            let alpha = (self.config.rate_func)(alpha);
            self.func.interpolate(&mut self.rabject, alpha);
            scene.insert_rabject(ctx, &self.rabject);
            scene.update_frame(ctx, dt);
            scene.frame_count += 1;
        }

        self.func.post_anim(&mut self.rabject);
        self.func.cleanup(&mut self.rabject, scene);

        if self.config.remove {
            scene.remove_rabject(&self.rabject);
            None
        } else {
            Some(self.rabject)
        }
    }
}
