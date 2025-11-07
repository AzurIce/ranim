//! Evaluation and animation
//!
//! The evaluation core of an animation is a `E: Eval<T>`.
//!
//! When constructing an animation, we need [`AnimationInfo`] besides the evaluation core, which is
//! [`AnimationCell<T>`].
//!
//! When satisfies `T: Extract<Target = CoreItem>`, [`AnimationCell<T>`] can be converted to a [`PrimitiveAnimationCell`].

use crate::{Extract, core_item::CoreItem, utils::rate_functions::linear};

use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct AnimationInfo {
    /// The rate function used for evaluating
    pub rate_func: fn(f64) -> f64,
    /// Show sec
    pub show_sec: f64,
    /// The duration seconds
    pub duration_secs: f64,
}

impl Default for AnimationInfo {
    fn default() -> Self {
        Self {
            rate_func: linear,
            show_sec: 0.0,
            duration_secs: 1.0,
        }
    }
}

impl AnimationInfo {
    pub fn range(&self) -> std::ops::Range<f64> {
        self.show_sec..self.show_sec + self.duration_secs
    }
    pub fn map_alpha(&self, alpha: f64) -> f64 {
        (self.rate_func)(alpha)
    }
    pub fn map_sec(&self, sec: f64) -> Option<f64> {
        if (self.show_sec..=self.show_sec + self.duration_secs).contains(&sec) {
            Some((sec - self.show_sec) / self.duration_secs)
        } else {
            None
        }
    }
}

impl AnimationInfo {
    /// A builder func to modify `show_sec`
    pub fn with_show_sec(mut self, show_sec: f64) -> Self {
        self.show_sec = show_sec;
        self
    }
    /// A builder func to modify `rate_func`
    pub fn with_rate_func(mut self, rate_func: fn(f64) -> f64) -> Self {
        self.rate_func = rate_func;
        self
    }
    /// A builder func to modify `secs`
    pub fn with_duration(mut self, secs: f64) -> Self {
        self.duration_secs = secs;
        self
    }
}

pub trait CoreItemAnimation {
    fn eval_alpha_core_item(&self, alpha: f64) -> Vec<CoreItem>;
    fn anim_info(&self) -> &AnimationInfo;
}

pub struct AnimationCell<T> {
    inner: Box<dyn Eval<T>>,
    pub info: AnimationInfo,
}

impl<T> AnimationCell<T> {
    pub fn with_show_sec(mut self, show_sec: f64) -> Self {
        self.info = self.info.with_show_sec(show_sec);
        self
    }
    pub fn with_rate_func(mut self, rate_func: fn(f64) -> f64) -> Self {
        self.info = self.info.with_rate_func(rate_func);
        self
    }
    pub fn with_duration(mut self, duration_secs: f64) -> Self {
        self.info = self.info.with_duration(duration_secs);
        self
    }
}

impl<T> Eval<T> for AnimationCell<T> {
    fn eval_alpha(&self, alpha: f64) -> T {
        self.inner.eval_alpha(alpha)
    }
}

impl<T: Extract<Target = CoreItem>> CoreItemAnimation for AnimationCell<T> {
    fn eval_alpha_core_item(&self, alpha: f64) -> Vec<CoreItem> {
        self.inner.eval_alpha(alpha).extract()
    }
    fn anim_info(&self) -> &AnimationInfo {
        &self.info
    }
}

// MARK: Eval
// ANCHOR: EvalDynamic
/// This is the core of any animation, an animation is basically a function on time.
///
/// This represents a normalized animation function for type `T`, which accepts
/// a progress value `alpha` in range [0, 1] and returns the evaluation result in type `T`.
pub trait Eval<T> {
    /// Evaluates at the given progress value `alpha` in range [0, 1].
    fn eval_alpha(&self, alpha: f64) -> T;
    /// Construct an [`AnimationCell<T>`] with default [`AnimationInfo`]
    fn into_animation_cell(self) -> AnimationCell<T>
    where
        Self: Sized + 'static,
    {
        AnimationCell {
            inner: Box::new(self),
            info: AnimationInfo::default(),
        }
    }
}

/// A static animation.
pub struct StaticAnim<T>(pub T);

impl<T: Clone> Eval<T> for StaticAnim<T> {
    fn eval_alpha(&self, _alpha: f64) -> T {
        self.0.clone()
    }
}

// ANCHOR_END: EvalDynamic
