use crate::{
    animation::{AnimationSpan, BasicRequirement, EvalDynamic, Evaluator},
    utils::rate_functions::smooth,
};

// MARK: Anim Trait
/// The methods to create animations for `T` that satisfies [`FuncRequirement`]
pub trait FuncAnim<T: BasicRequirement + 'static> {
    /// Create a [`Func`] anim.
    fn func(self, f: impl Fn(&T, f64) -> T + Send + Sync + 'static) -> AnimationSpan<T>;
}

impl<T: BasicRequirement + 'static> FuncAnim<T> for T {
    fn func(self, f: impl Fn(&T, f64) -> T + Send + Sync + 'static) -> AnimationSpan<T> {
        AnimationSpan::from_evaluator(Evaluator::new_dynamic(Func::new(self.clone(), f)))
            .with_rate_func(smooth)
    }
}

// MARK: Impl
/// An func anim.
///
/// This simply use the given func to eval the animation state.
pub struct Func<T: BasicRequirement> {
    src: T,
    #[allow(clippy::type_complexity)]
    f: Box<dyn Fn(&T, f64) -> T + Send + Sync>,
}

impl<T: BasicRequirement> Func<T> {
    /// Constructor
    pub fn new(target: T, f: impl Fn(&T, f64) -> T + Send + Sync + 'static) -> Self {
        Self {
            src: target,
            f: Box::new(f),
        }
    }
}

impl<T: BasicRequirement> EvalDynamic<T> for Func<T> {
    fn eval_alpha(&self, alpha: f64) -> T {
        (self.f)(&self.src, alpha)
    }
}
