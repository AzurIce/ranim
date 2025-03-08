use super::{AnimSchedule, EvalDynamic, Schedule};
use crate::items::Rabject;
use crate::prelude::Interpolatable;
use crate::utils::rate_functions::smooth;

// MARK: Require Trait
pub trait FadingRequirement: Opacity + Interpolatable + Clone {}
impl<T: Opacity + Interpolatable + Clone> FadingRequirement for T {}

// MARK: Anim Trait
pub trait FadingAnim<'r, 't, T: FadingRequirement + 'static> {
    fn fade_in(&'r mut self) -> AnimSchedule<'r, 't, T>;
    fn fade_out(&'r mut self) -> AnimSchedule<'r, 't, T>;
}

impl<'r, 't, T: FadingRequirement + 'static> FadingAnim<'r, 't, T> for Rabject<'t, T> {
    fn fade_in(&'r mut self) -> AnimSchedule<'r, 't, T> {
        FadeIn::new(self.data.clone())
            .schedule(self)
            .with_rate_func(smooth)
    }

    fn fade_out(&'r mut self) -> AnimSchedule<'r, 't, T> {
        FadeOut::new(self.data.clone())
            .schedule(self)
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
    fn eval_alpha(&self, alpha: f32) -> T {
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
    fn eval_alpha(&self, alpha: f32) -> T {
        self.src.lerp(&self.dst, alpha)
    }
}

pub trait Opacity {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self;
}
