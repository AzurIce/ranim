use crate::items::{Entity, Rabject};
use crate::prelude::Interpolatable;

use super::{AnimScheduler, EntityAnim, PureEvaluator};

pub trait Fading: Opacity + Interpolatable + Entity {}
impl<T: Opacity + Interpolatable + Entity> Fading for T {}

pub trait FadingAnim<'r, 't, T: Fading + 'static> {
    fn fade_in(&'r mut self) -> AnimScheduler<'r, 't, T, EntityAnim<T>>;
    fn fade_out(&'r mut self) -> AnimScheduler<'r, 't, T, EntityAnim<T>>;
}

impl<'r, 't, T: Fading + 'static> FadingAnim<'r, 't, T> for Rabject<'t, T> {
    fn fade_in(&'r mut self) -> AnimScheduler<'r, 't, T, EntityAnim<T>> {
        let func = FadeIn::new(self.data.clone());
        AnimScheduler::new(self, EntityAnim::new(self.id, self.data.clone(), func))
    }

    fn fade_out(&'r mut self) -> AnimScheduler<'r, 't, T, EntityAnim<T>> {
        let func = FadeOut::new(self.data.clone());
        AnimScheduler::new(self, EntityAnim::new(self.id, self.data.clone(), func))
    }
}

// ---------------------------------------------------- //

pub struct FadeIn<T: Fading> {
    src: T,
    dst: T,
}

impl<T: Fading> FadeIn<T> {
    fn new(target: T) -> Self {
        let mut src = target.clone();
        let dst = target.clone();
        src.set_opacity(0.0);
        Self { src, dst }
    }
}

impl<T: Fading> PureEvaluator<T> for FadeIn<T> {
    fn eval_alpha(&self, alpha: f32) -> T {
        self.src.lerp(&self.dst, alpha)
    }
}

pub struct FadeOut<T: Fading> {
    src: T,
    dst: T,
}

impl<T: Fading> FadeOut<T> {
    fn new(target: T) -> Self {
        let src = target.clone();
        let mut dst = target.clone();
        dst.set_opacity(0.0);
        Self { src, dst }
    }
}

impl<T: Fading> PureEvaluator<T> for FadeOut<T> {
    fn eval_alpha(&self, alpha: f32) -> T {
        self.src.lerp(&self.dst, alpha)
    }
}

pub trait Opacity {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self;
}
