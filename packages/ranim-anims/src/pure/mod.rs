//! Pure (closed-form, stateless) evaluation: the [`PureEval`](crate::pure::PureEval)
//! capability trait and its [`Pure`](crate::pure::Pure) adapter into the
//! general [`Eval`](ranim_core::animation::Eval) protocol.
//!
//! This module also hosts all built-in pure animation families
//! ([`camera`](crate::pure::camera), [`creation`](crate::pure::creation),
//! [`fading`](crate::pure::fading), [`func`](crate::pure::func),
//! [`morph`](crate::pure::morph), [`rotating`](crate::pure::rotating)).

use ranim_core::{
    animation::Eval,
    time::{DeltaTime, Time},
};

/// Camera frame animation
pub mod camera;
/// Creation animation
pub mod creation;
/// Fading animation
pub mod fading;
/// Func animation
pub mod func;
/// Morph animation
pub mod morph;
/// Rotating animation
pub mod rotating;

/// The capability of a closed-form, stateless evaluation from progress.
///
/// This is what pure animation types implement: a single method with no
/// defaults. Closures `Fn(f64) -> T` implement it automatically, so
/// `Pure(|alpha| ...)` is the lightweight way to write a pure segment.
///
/// Wrap a `PureEval` in [`Pure`] to turn it into a full animation segment.
pub trait PureEval {
    /// Value produced by this evaluator.
    type Output;

    /// Evaluate the animation at a normalized progress in `[0, 1]`.
    fn eval_alpha(&self, alpha: f64) -> Self::Output;
}

impl<T, F> PureEval for F
where
    F: Fn(f64) -> T,
{
    type Output = T;

    fn eval_alpha(&self, alpha: f64) -> Self::Output {
        (self)(alpha)
    }
}

/// Adapter turning a [`PureEval`] into the general [`Eval`] protocol.
///
/// A pure segment is a stateless specialization of the general protocol:
/// `sample` is the closed form evaluated at `time.alpha`, and `reset`/`step`
/// are trivially empty (there is no state to advance or restore).
///
/// ```rust,ignore
/// // A pure segment from a closure:
/// let animation = Pure(|alpha| Square::new(alpha)).with_duration(2.0);
/// // A pure segment from a named type:
/// let animation = Pure(FadeIn::new(square)).with_duration(1.0);
/// ```
pub struct Pure<E>(pub E);

impl<E: PureEval> Eval for Pure<E> {
    type Output = E::Output;

    fn sample(&self, time: &Time) -> Self::Output {
        self.0.eval_alpha(time.alpha)
    }

    fn reset(&mut self) {}

    fn step(&mut self, _time: &Time, _delta_time: &DeltaTime) {}
}
