//! Evaluation and animation
//!
//! The evaluation core of an animation is a `E: Eval<T>`.
//!
//! When constructing an animation, we need [`AnimationInfo`] besides the evaluation core, which is
//! [`AnimationCell<T>`].
//!
//! When satisfies `T: Extract<Target = CoreItem>`, [`AnimationCell<T>`] can be converted to a [`CoreItemAnimation`].

use crate::{
    core_item::{AnyExtractCoreItem, CoreItem, DynItem},
    utils::rate_functions::linear,
};

use std::fmt::Debug;

/// Info of an animation.
///
/// When [`AnimationInfo::enabled`] is `false`, the animation will not be evaluated.
#[derive(Debug, Clone)]
pub struct AnimationInfo {
    /// The rate function used for evaluating, default value: [`linear`]
    pub rate_func: fn(f64) -> f64,
    /// Show sec, default value: 0.0
    pub at_sec: f64,
    /// The duration seconds, default value: 1.0
    pub duration_secs: f64,
    /// Is enabled, default value: true
    pub enabled: bool,
}

impl Default for AnimationInfo {
    fn default() -> Self {
        Self {
            rate_func: linear,
            at_sec: 0.0,
            duration_secs: 1.0,
            enabled: true,
        }
    }
}

impl AnimationInfo {
    /// The time range of the animation
    pub fn range(&self) -> std::ops::Range<f64> {
        self.at_sec..self.at_sec + self.duration_secs
    }
    /// Map the global sec to alpha
    pub fn map_sec(&self, sec: f64) -> Option<f64> {
        if (self.at_sec..=self.at_sec + self.duration_secs).contains(&sec) {
            Some((self.rate_func)((sec - self.at_sec) / self.duration_secs))
        } else {
            None
        }
    }
}

impl AnimationInfo {
    /// A builder func to modify [`AnimationInfo::at_sec`]
    pub fn at(mut self, at_sec: f64) -> Self {
        self.at_sec = at_sec;
        self
    }
    /// A builder func to modify [`AnimationInfo::rate_func`]
    pub fn with_rate_func(mut self, rate_func: fn(f64) -> f64) -> Self {
        self.rate_func = rate_func;
        self
    }
    /// A builder func to modify [`AnimationInfo::duration_secs`]
    pub fn with_duration(mut self, secs: f64) -> Self {
        self.duration_secs = secs;
        self
    }
    /// A builder func to modify [`AnimationInfo::enabled`]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Animation of core items.
pub trait CoreItemAnimation {
    /// Evaluate to [`DynItem`]
    fn eval_alpha_dyn(&self, alpha: f64) -> DynItem;
    /// Evaluate to [`CoreItem`]s
    fn eval_alpha_core_item(&self, alpha: f64) -> Vec<CoreItem>;
    /// Get the animation info
    fn anim_info(&self) -> &AnimationInfo;
    /// Get the name of the animation
    fn anim_name(&self) -> &str;
}

/// A cell of an animation
pub struct AnimationCell<T> {
    inner: Box<dyn Eval<T>>,
    /// The animation info
    pub info: AnimationInfo,
    anim_name: String,
}

impl<T> AnimationCell<T> {
    /// A builder func to modify [`AnimationInfo::at_sec`]
    pub fn at(mut self, at_sec: f64) -> Self {
        self.info = self.info.at(at_sec);
        self
    }
    /// A builder func to modify [`AnimationInfo::rate_func`]
    pub fn with_rate_func(mut self, rate_func: fn(f64) -> f64) -> Self {
        self.info = self.info.with_rate_func(rate_func);
        self
    }
    /// A builder func to modify [`AnimationInfo::duration_secs`]
    pub fn with_duration(mut self, duration_secs: f64) -> Self {
        self.info = self.info.with_duration(duration_secs);
        self
    }
    /// A builder func to modify [`AnimationInfo::enabled`]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.info = self.info.with_enabled(enabled);
        self
    }
}

impl<T> Eval<T> for AnimationCell<T> {
    fn eval_alpha(&self, alpha: f64) -> T {
        self.inner.eval_alpha(alpha)
    }
}

impl<T: AnyExtractCoreItem> CoreItemAnimation for AnimationCell<T> {
    fn eval_alpha_dyn(&self, alpha: f64) -> DynItem {
        DynItem(Box::new(self.inner.eval_alpha(alpha)))
    }
    fn eval_alpha_core_item(&self, alpha: f64) -> Vec<CoreItem> {
        self.inner.eval_alpha(alpha).extract()
    }
    fn anim_info(&self) -> &AnimationInfo {
        &self.info
    }
    fn anim_name(&self) -> &str {
        &self.anim_name
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
            anim_name: std::any::type_name::<Self>().to_string(),
        }
    }
}

// MARK: StaticAnim
/// The requirement for [`StaticAnim`]
pub trait StaticAnimRequirement: Clone {}

impl<T: Clone> StaticAnimRequirement for T {}

/// The helper methods for [`Static`], i.e. evaluates to the same value
pub trait StaticAnim: StaticAnimRequirement {
    /// Show the item
    fn show(&self) -> AnimationCell<Self>;
    /// Hide the item
    fn hide(&self) -> AnimationCell<Self>;
}

impl<T: StaticAnimRequirement + 'static> StaticAnim for T {
    fn show(&self) -> AnimationCell<Self> {
        Static(self.clone())
            .into_animation_cell()
            .with_duration(0.0)
    }
    fn hide(&self) -> AnimationCell<Self> {
        Static(self.clone())
            .into_animation_cell()
            .with_enabled(false)
            .with_duration(0.0)
    }
}

/// A static animation.
pub struct Static<T>(pub T);

impl<T: Clone> Eval<T> for Static<T> {
    fn eval_alpha(&self, _alpha: f64) -> T {
        self.0.clone()
    }
}

// ANCHOR_END: EvalDynamic
