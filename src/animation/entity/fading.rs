use crate::{items::Entity, Rabject};
use crate::prelude::Interpolatable;

use crate::animation::{AnimationFunc, entity::EntityAnimation};

pub fn fade_in<T: Opacity + Interpolatable + 'static + Entity>(rabject: Rabject<T>) -> EntityAnimation<T> {
    let func = FadeIn::new(rabject.inner.clone());
    EntityAnimation::new(rabject, func)
}

pub fn fade_out<T: Opacity + Interpolatable + 'static + Entity>(rabject: Rabject<T>) -> EntityAnimation<T> {
    let func = FadeOut::new(rabject.inner.clone());
    EntityAnimation::new(rabject, func)
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

impl<T: Entity + Interpolatable + Opacity> AnimationFunc<T> for FadeIn<T> {
    fn eval_alpha(&mut self, target: &mut T, alpha: f32) {
        *target = self.src.lerp(&self.dst, alpha)
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

impl<T: Entity + Interpolatable + Opacity> AnimationFunc<T> for FadeOut<T> {
    fn eval_alpha(&mut self, target: &mut T, alpha: f32) {
        *target = self.src.lerp(&self.dst, alpha)
    }
}

// pub enum FadingType {
//     Out,
//     In,
// }

// pub struct Fading<R: Opacity + Interpolatable + Clone> {
//     pub(crate) src: Option<R>,
//     pub(crate) dst: Option<R>,
//     pub(crate) fading_type: FadingType,
// }

pub trait Opacity {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self;
}

// impl<T: Opacity + Interpolatable + Clone> AnimationFunc<T> for Fading<T> {
//     fn init(&mut self, rabject: &mut T) {
//         self.src = Some(rabject.clone());
//         self.dst = Some(rabject.clone());
//         match self.fading_type {
//             FadingType::Out => self.dst.as_mut(),
//             FadingType::In => self.src.as_mut(),
//         }
//         .unwrap()
//         .set_opacity(0.0);
//     }

//     fn interpolate(&mut self, entity: &mut T, alpha: f32) {
//         entity.update_from(
//             &self
//                 .src
//                 .as_ref()
//                 .unwrap()
//                 .lerp(self.dst.as_ref().unwrap(), alpha),
//         );
//     }

//     fn post_anim(&mut self, entity: &mut T) {
//         entity.update_from(self.dst.as_ref().unwrap());
//     }
// }
