//! Pure (closed-form) evaluation adapters.

use super::Eval;

/// Adapter turning a raw closure `Fn(f64) -> T` into an [`Eval`] segment.
///
/// A closure is an anonymous type, so it cannot implement `Eval` by name; this
/// named wrapper is the lightweight way to write a pure segment from a closure:
///
/// ```rust,ignore
/// let animation = PureFunc::new(|alpha| Square::new(alpha)).with_duration(2.0);
/// ```
///
/// Named pure animations implement `Eval` directly and do not need this
/// wrapper.
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
