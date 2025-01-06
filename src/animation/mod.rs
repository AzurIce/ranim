pub mod fading;
pub mod transform;
pub mod creation;

use std::time;

use crate::{scene::{Entity, EntityId}, updater::Updater, utils::rate_functions::smooth};

#[allow(unused)]
use log::trace;

pub struct Enter;



pub struct AnimationConfig {
    pub run_time: time::Duration,
    pub rate_func: Box<dyn Fn(f32) -> f32>,
    pub lag_ratio: Option<f32>,
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
            lag_ratio: None,
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
    pub fn set_lag_ratio(&mut self, lag_ratio: Option<f32>) -> &mut Self {
        self.lag_ratio = lag_ratio;
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
    fn init(&mut self, entity: &mut T);

    fn interpolate(&mut self, entity: &mut T, alpha: f32);

    #[allow(unused)]
    fn post_anim(&mut self, entity: &mut T) {}
}

// /// An [`AnimationAction`] specifies what to do with the animatied entity
// pub enum AnimationAction<T: Entity + 'static> {
//     /// Insert the entity into the scene, and animate it
//     InsertAndAnimate(T),
//     /// Animate the entity, and remove it from the scene
//     AnimateAndRemove(EntityId<T>),
//     /// Just animate
//     Animate(EntityId<T>),
// }

pub enum AnimateTarget<T: Entity + 'static> {
    Insert(T),
    Existed(EntityId<T>),
    // Remove(EntityId<T>),
}

impl<T: Entity + 'static> From<T> for AnimateTarget<T> {
    fn from(value: T) -> Self {
        Self::Insert(value)
    }
}

// impl<'a, T: Entity + 'static> From<&'a EntityId<T>> for AnimateTarget<'a, T> {
//     fn from(value: &'a EntityId<T>) -> Self {
//         Self::Existed(value)
//     }
// }
impl<T: Entity + 'static> From<EntityId<T>> for AnimateTarget<T> {
    fn from(value: EntityId<T>) -> Self {
        Self::Existed(value)
    }
}

/// A [`Updater`] wraps an [`AnimationFunc`]
///
/// See [`AnimationFunc`]
pub struct Animation<T> {
    acc_t: f32,

    // action: AnimationAction<T>,
    pub func: Box<dyn AnimationFunc<T>>,
    pub(crate) config: AnimationConfig,
}

impl<T> Updater<T> for Animation<T> {
    fn on_create(&mut self, entity: &mut T) {
        self.func.init(entity);
    }
    fn on_update(&mut self, entity: &mut T, dt: f32) -> bool {
        self.acc_t += dt;
        let alpha = (self.acc_t / self.config.run_time.as_secs_f32()).clamp(0.0, 1.0);

        let alpha = (self.config.rate_func)(alpha);

        self.func.interpolate(entity, alpha);
        self.acc_t <= self.config.run_time.as_secs_f32()
    }
    fn on_destroy(&mut self, entity: &mut T) {
        self.func.post_anim(entity);
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
