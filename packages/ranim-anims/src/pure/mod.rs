//! Pure (closed-form, stateless) animation families.
//!
//! Every named pure animation (fade, morph, create, camera, rotate, func) now
//! implements [`Eval`](ranim_core::animation::Eval) directly — a pure segment
//! is just an `Eval` whose `eval_alpha` is a closed form. The only wrapper
//! left is [`PureFunc`], the adapter that turns a raw closure
//! `Fn(f64) -> T` into an `Eval` (needed because a closure cannot implement
//! `Eval` directly under the orphan rule — `Eval` lives in ranim-core).
//!
//! This module hosts the built-in pure animation families
//! ([`camera`](crate::pure::camera), [`creation`](crate::pure::creation),
//! [`fading`](crate::pure::fading), [`func`](crate::pure::func),
//! [`morph`](crate::pure::morph), [`rotating`](crate::pure::rotating)).

use ranim_core::animation::Eval;

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

/// Adapter turning a raw closure `Fn(f64) -> T` into an [`Eval`] segment.
///
/// A closure cannot implement `Eval` directly (orphan rule — `Eval` lives in
/// ranim-core, and the closure type is anonymous), so this named wrapper is the
/// lightweight way to write a pure segment from a closure:
///
/// ```rust,ignore
/// let animation = PureFunc::new(|alpha| Square::new(alpha)).with_duration(2.0);
/// ```
///
/// Named pure animations (`FadeIn`, `Morph`, `Create`, ...) implement
/// `Eval` directly and do not need this wrapper.
pub struct PureFunc<F>(pub F);

impl<F> PureFunc<F> {
    /// Wrap a closure into an `Eval` animation segment.
    pub fn new(f: F) -> Self {
        Self(f)
    }
}

impl<T, F> Eval for PureFunc<F>
where
    F: Fn(f64) -> T,
{
    type Output = T;

    fn eval_alpha(&self, alpha: f64) -> T {
        (self.0)(alpha)
    }
}
