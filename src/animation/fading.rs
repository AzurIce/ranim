use crate::rabject::Rabject;

use super::{Animation, AnimationFunc};

pub enum Fading {
    Out,
    In,
}

impl Fading {
    pub fn fade_in<R: Rabject + Opacity>() -> Animation<R> {
        Animation::new(Self::In)
    }

    pub fn fade_out<R: Rabject + Opacity>() -> Animation<R> {
        Animation::new(Self::Out)
    }
}

pub trait Opacity {
    fn set_opacity(&mut self, opacity: f32);
}

impl<R: Rabject + Opacity> AnimationFunc<R> for Fading {
    fn pre_anim(&mut self, rabject: &mut R) {
        match self {
            Fading::Out => rabject.set_opacity(1.0),
            Fading::In => rabject.set_opacity(0.0),
        };
    }

    fn interpolate(&mut self, rabject: &mut R, alpha: f32) {
        match self {
            Fading::Out => rabject.set_opacity(1.0 - alpha),
            Fading::In => rabject.set_opacity(alpha),
        };
    }
}
