use super::{AnimationSpan, EvalDynamic};
use crate::animation::Evaluator;
use crate::traits::{Interpolatable, Opacity};
use crate::utils::rate_functions::smooth;

// MARK: Require Trait
pub trait FadingRequirement: Opacity + Interpolatable + Clone {}
impl<T: Opacity + Interpolatable + Clone> FadingRequirement for T {}

// MARK: Anim Trait
pub trait FadingAnim<T: FadingRequirement + 'static> {
    fn fade_in(self) -> AnimationSpan<T>;
    fn fade_out(self) -> AnimationSpan<T>;
}

impl<T: FadingRequirement + 'static> FadingAnim<T> for T {
    fn fade_in(self) -> AnimationSpan<T> {
        AnimationSpan::from_evaluator(Evaluator::new_dynamic(FadeIn::new(self.clone())))
            .with_rate_func(smooth)
    }
    fn fade_out(self) -> AnimationSpan<T> {
        AnimationSpan::from_evaluator(Evaluator::new_dynamic(FadeOut::new(self.clone())))
            .with_rate_func(smooth)
    }
}

// MARK: Impl

pub struct FadeIn<T: FadingRequirement> {
    src: T,
    dst: T,
}

impl<T: FadingRequirement> FadeIn<T> {
    fn new(target: T) -> Self {
        let mut src = target.clone();
        let dst = target.clone();
        src.set_opacity(0.0);
        Self { src, dst }
    }
}

impl<T: FadingRequirement> EvalDynamic<T> for FadeIn<T> {
    fn eval_alpha(&self, alpha: f64) -> T {
        self.src.lerp(&self.dst, alpha)
    }
}

pub struct FadeOut<T: FadingRequirement> {
    src: T,
    dst: T,
}

impl<T: FadingRequirement> FadeOut<T> {
    fn new(target: T) -> Self {
        let src = target.clone();
        let mut dst = target.clone();
        dst.set_opacity(0.0);
        Self { src, dst }
    }
}

impl<T: FadingRequirement> EvalDynamic<T> for FadeOut<T> {
    fn eval_alpha(&self, alpha: f64) -> T {
        self.src.lerp(&self.dst, alpha)
    }
}
