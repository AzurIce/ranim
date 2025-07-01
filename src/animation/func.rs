use crate::{
    animation::{AnimationSpan, EvalDynamic, Evaluator},
    utils::rate_functions::smooth,
};

// MARK: Require Trait
pub trait FuncRequirement: Clone {}

// MARK: Anim Trait
pub trait FuncAnim<T: FuncRequirement + 'static> {
    fn func(self, f: impl Fn(&T, f64) -> T + 'static) -> AnimationSpan<T>;
}

impl<T: FuncRequirement + 'static> FuncAnim<T> for T {
    fn func(self, f: impl Fn(&T, f64) -> T + 'static) -> AnimationSpan<T> {
        AnimationSpan::from_evaluator(Evaluator::new_dynamic(Func::new(self.clone(), f)))
            .with_rate_func(smooth)
    }
}

// MARK: Impl
pub struct Func<T: FuncRequirement> {
    src: T,
    #[allow(clippy::type_complexity)]
    f: Box<dyn Fn(&T, f64) -> T>,
}

impl<T: FuncRequirement> Func<T> {
    pub fn new(target: T, f: impl Fn(&T, f64) -> T + 'static) -> Self {
        Self {
            src: target,
            f: Box::new(f),
        }
    }
}

impl<T: FuncRequirement> EvalDynamic<T> for Func<T> {
    fn eval_alpha(&self, alpha: f64) -> T {
        (self.f)(&self.src, alpha)
    }
}
