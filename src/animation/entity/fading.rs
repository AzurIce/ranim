use crate::items::{Entity, Rabject};
use crate::prelude::Interpolatable;

use crate::animation::entity::{EntityAnim, PureEvaluator};

use crate::animation::AnimWithParams;

pub fn fade_in<T: Opacity + Interpolatable + 'static + Entity>(
    rabject: &Rabject<T>,
) -> AnimWithParams<EntityAnim<T>> {
    let func = FadeIn::new(rabject.data.clone());
    AnimWithParams::new(EntityAnim::new(rabject.clone(), func))
}

pub fn fade_out<T: Opacity + Interpolatable + 'static + Entity>(
    rabject: &Rabject<T>,
) -> AnimWithParams<EntityAnim<T>> {
    let func = FadeOut::new(rabject.data.clone());
    AnimWithParams::new(EntityAnim::new(rabject.clone(), func))
}

// ---------------------------------------------------- //

pub struct FadeIn<T: Entity + Interpolatable + Opacity> {
    src: T,
    dst: T,
}

impl<T: Entity + Interpolatable + Opacity + Clone> FadeIn<T> {
    pub fn new(target: T) -> Self {
        let mut src = target.clone();
        let dst = target.clone();
        src.set_opacity(0.0);
        Self { src, dst }
    }
}

impl<T: Entity + Interpolatable + Opacity> PureEvaluator<T> for FadeIn<T> {
    fn eval_alpha(&self, alpha: f32) -> T {
        self.src.lerp(&self.dst, alpha)
    }
}

pub struct FadeOut<T: Entity + Interpolatable + Opacity> {
    src: T,
    dst: T,
}

impl<T: Entity + Interpolatable + Opacity + Clone> FadeOut<T> {
    pub fn new(target: T) -> Self {
        let src = target.clone();
        let mut dst = target.clone();
        dst.set_opacity(0.0);
        Self { src, dst }
    }
}

impl<T: Entity + Interpolatable + Opacity> PureEvaluator<T> for FadeOut<T> {
    fn eval_alpha(&self, alpha: f32) -> T {
        self.src.lerp(&self.dst, alpha)
    }
}

pub trait Opacity {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self;
}
