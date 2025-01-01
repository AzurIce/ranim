pub mod fading;
pub mod transform;

use std::time;

use crate::{updater::Updater, utils::rate_functions::smooth};

#[allow(unused)]
use log::trace;

pub struct AnimationConfig {
    pub run_time: time::Duration,
    pub rate_func: Box<dyn Fn(f32) -> f32>,
    // /// Whether the mobject will be removed from the scene after the animation
    // pub(crate) remove: bool,
    // /// The rabject will be inserted into the scene after the animation
    // pub(crate) end_rabject: Option<RabjectWithId<R>>,
}

impl Default for AnimationConfig {
    /// Default animation config
    /// - run_time: 1.0s
    /// - rate_func: smooth
    /// - remove: false
    /// - end_rabject: None
    fn default() -> Self {
        Self {
            run_time: time::Duration::from_secs_f32(1.0),
            rate_func: Box::new(smooth),
            // remove: false,
            // end_rabject: None,
        }
    }
}

impl AnimationConfig {
    pub fn set_run_time(&mut self, run_time: time::Duration) -> &mut Self {
        self.run_time = run_time;
        self
    }
    pub fn set_rate_func(&mut self, rate_func: Box<dyn Fn(f32) -> f32>) -> &mut Self {
        self.rate_func = rate_func;
        self
    }
    // pub fn set_remove(&mut self, remove: bool) -> &mut Self {
    //     self.remove = remove;
    //     self
    // }
    // pub fn set_end_rabject(&mut self, end_rabject: Option<RabjectWithId<R>>) -> &mut Self {
    //     self.end_rabject = end_rabject;
    //     self
    // }

    pub fn calc_frames(&self, fps: f32) -> usize {
        (self.run_time.as_secs_f32() * fps).ceil() as usize
    }
}

/// A trait representing an animation function
///
/// The main difference between [`AnimationFunc`] and [`Updater`] is that
/// the [`Updater`]'s parameter is elapsed time `dt` but
/// the [`AnimationFunc`]'s parameter is the animation progress `alpha`.
///
/// The `alpha` is calculated by the wrapper [`Animation`],
/// and the [`Animation`] is actually just an [`Updater`].
///
/// See [`Animation`], [`Updater`]
pub trait AnimationFunc<T> {
    #[allow(unused)]
    fn pre_anim(&mut self, rabject: &mut T) {}

    fn interpolate(&mut self, rabject: &mut T, alpha: f32);

    #[allow(unused)]
    fn post_anim(&mut self, rabject: &mut T) {}
}

/// A [`Updater`] wraps an [`AnimationFunc`]
///
/// See [`AnimationFunc`]
pub struct Animation<T> {
    acc_t: f32,

    pub func: Box<dyn AnimationFunc<T>>,
    pub(crate) config: AnimationConfig,
}

impl<T> Updater<T> for Animation<T> {
    fn on_create(&mut self, rabject: &mut T) {
        self.func.pre_anim(rabject);
    }
    fn on_update(&mut self, rabject: &mut T, dt: f32) -> bool {
        self.acc_t += dt;
        let alpha = (self.acc_t / self.config.run_time.as_secs_f32()).clamp(0.0, 1.0);

        let alpha = (self.config.rate_func)(alpha);

        self.func.interpolate(rabject, alpha);
        self.acc_t <= self.config.run_time.as_secs_f32()
    }
    fn on_destroy(&mut self, rabject: &mut T) {
        self.func.post_anim(rabject);
    }
}

impl<T> Animation<T> {
    pub fn new(func: impl AnimationFunc<T> + 'static) -> Self {
        Self {
            func: Box::new(func),
            config: Default::default(),
            acc_t: 0.0,
        }
    }

    pub fn with_config(mut self, config: AnimationConfig) -> Self {
        self.config = config;
        self
    }

    pub fn config(mut self, config: impl FnOnce(&mut AnimationConfig)) -> Self {
        config(&mut self.config);
        self
    }
}
