pub mod fading;
pub mod transform;

use std::{ops::Deref, time};

use log::trace;

use crate::{
    rabject::{Rabject, RabjectWithId},
    scene::Scene,
    utils::rate_functions::smooth,
    RanimContext,
};

pub struct AnimationConfig<R: Rabject> {
    pub(crate) run_time: time::Duration,
    pub(crate) rate_func: Box<dyn Fn(f32) -> f32>,
    /// Whether the mobject will be removed from the scene after the animation
    pub(crate) remove: bool,
    /// The rabject will be inserted into the scene after the animation
    pub(crate) end_rabject: Option<RabjectWithId<R>>,
}

impl<R: Rabject> Default for AnimationConfig<R> {
    /// Default animation config
    /// - run_time: 1.0s
    /// - rate_func: smooth
    /// - remove: false
    /// - end_rabject: None
    fn default() -> Self {
        Self {
            run_time: time::Duration::from_secs_f32(1.0),
            rate_func: Box::new(smooth),

            remove: false,
            end_rabject: None,
        }
    }
}

impl<R: Rabject> AnimationConfig<R> {
    pub fn set_run_time(&mut self, run_time: time::Duration) -> &mut Self {
        self.run_time = run_time;
        self
    }
    pub fn set_rate_func(&mut self, rate_func: Box<dyn Fn(f32) -> f32>) -> &mut Self {
        self.rate_func = rate_func;
        self
    }
    pub fn set_remove(&mut self, remove: bool) -> &mut Self {
        self.remove = remove;
        self
    }
    pub fn set_end_rabject(&mut self, end_rabject: Option<RabjectWithId<R>>) -> &mut Self {
        self.end_rabject = end_rabject;
        self
    }

    pub fn calc_frames(&self, fps: f32) -> usize {
        (self.run_time.as_secs_f32() * fps).ceil() as usize
    }
}

pub trait AnimationFunc<R: Rabject> {
    #[allow(unused)]
    fn prepare(&mut self, rabject: &mut RabjectWithId<R>, scene: &mut Scene) {}
    #[allow(unused)]
    fn pre_anim(&mut self, rabject: &mut RabjectWithId<R>) {}

    fn interpolate(&mut self, rabject: &mut RabjectWithId<R>, alpha: f32);

    #[allow(unused)]
    fn post_anim(&mut self, rabject: &mut RabjectWithId<R>) {}

    #[allow(unused)]
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
    pub(crate) rabject: RabjectWithId<R>,

    pub func: Box<dyn AnimationFunc<R>>,
    pub(crate) config: AnimationConfig<R>,
}

impl<R: Rabject> Animation<R> {
    pub fn new(rabject: RabjectWithId<R>, func: impl AnimationFunc<R> + 'static) -> Self {
        Self {
            rabject,
            func: Box::new(func),
            config: Default::default(),
        }
    }

    pub fn with_config(mut self, config: AnimationConfig<R>) -> Self {
        self.config = config;
        self
    }

    pub fn config(mut self, config: impl FnOnce(&mut AnimationConfig<R>)) -> Self {
        config(&mut self.config);
        self
    }

    pub fn play(mut self, ctx: &mut RanimContext, scene: &mut Scene) -> Option<RabjectWithId<R>> {
        trace!(
            "[Animation] Playing animation on {:?}...",
            self.rabject.id()
        );
        self.func.prepare(&mut self.rabject, scene);
        self.func.pre_anim(&mut self.rabject);
        scene.insert_rabject(ctx, &self.rabject);

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
        scene.insert_rabject(ctx, &self.rabject);
        self.func.cleanup(&mut self.rabject, scene);

        if let Some(end_rabject) = self.config.end_rabject {
            scene.remove_rabject(&self.rabject);
            scene.insert_rabject(ctx, &end_rabject);
            return Some(end_rabject);
        }

        if self.config.remove {
            scene.remove_rabject(&self.rabject);
            None
        } else {
            Some(self.rabject)
        }
    }
}
