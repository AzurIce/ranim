use super::{AnimSchedule, AnimationSpan, EvalDynamic, ToEvaluator};
use crate::items::Rabject;
use crate::traits::{Interpolatable, Opacity};
use crate::utils::rate_functions::smooth;

// MARK: Require Trait
pub trait FadingRequirement: Opacity + Interpolatable + Clone {}
impl<T: Opacity + Interpolatable + Clone> FadingRequirement for T {}

// MARK: Anim Trait
pub trait FadingAnim<T: FadingRequirement + 'static> {
    fn fade_in(&self) -> AnimationSpan<T>;
    fn fade_out(&self) -> AnimationSpan<T>;
}

pub trait FadingAnimSchedule<'r, T: FadingRequirement + 'static> {
    fn fade_in(&'r mut self) -> AnimSchedule<'r, T>;
    fn fade_out(&'r mut self) -> AnimSchedule<'r, T>;
}

impl<T: FadingRequirement + 'static> FadingAnim<T> for T {
    fn fade_in(&self) -> AnimationSpan<T> {
        AnimationSpan::from_evaluator(FadeIn::new(self.clone()).to_evaluator())
            .with_rate_func(smooth)
    }
    fn fade_out(&self) -> AnimationSpan<T> {
        AnimationSpan::from_evaluator(FadeOut::new(self.clone()).to_evaluator())
            .with_rate_func(smooth)
    }
}

impl<'r, T: FadingRequirement + 'static> FadingAnimSchedule<'r, T> for Rabject<T> {
    fn fade_in(&'r mut self) -> AnimSchedule<'r, T> {
        AnimSchedule::new(self, self.data.fade_in())
    }

    fn fade_out(&'r mut self) -> AnimSchedule<'r, T> {
        AnimSchedule::new(self, self.data.fade_out())
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
