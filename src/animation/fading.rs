use crate::{prelude::Interpolatable, rabject::Updatable};

use super::{Animation, AnimationFunc};

pub enum FadingType {
    Out,
    In,
}

pub struct Fading<R: Opacity + Interpolatable + Clone> {
    pub(crate) src: Option<R>,
    pub(crate) dst: Option<R>,
    pub(crate) fading_type: FadingType,
}

impl<R: Opacity + Interpolatable + Clone + 'static> Fading<R> {
    pub fn fade_in() -> Animation<R> {
        Animation::new(Self {
            src: None,
            dst: None,
            fading_type: FadingType::In,
        })
    }

    pub fn fade_out() -> Animation<R> {
        Animation::new(Self {
            src: None,
            dst: None,
            fading_type: FadingType::Out,
        })
    }
}

pub trait Opacity {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self;
}

impl<T: Opacity + Interpolatable + Clone> AnimationFunc<T> for Fading<T> {
    fn init(&mut self, rabject: &mut T) {
        self.src = Some(rabject.clone());
        self.dst = Some(rabject.clone());
        match self.fading_type {
            FadingType::Out => self.dst.as_mut(),
            FadingType::In => self.src.as_mut(),
        }
        .unwrap()
        .set_opacity(0.0);
    }

    fn interpolate(&mut self, entity: &mut T, alpha: f32) {
        entity.update_from(
            &self
                .src
                .as_ref()
                .unwrap()
                .lerp(self.dst.as_ref().unwrap(), alpha),
        );
    }

    fn post_anim(&mut self, entity: &mut T) {
        entity.update_from(self.dst.as_ref().unwrap());
    }
}
