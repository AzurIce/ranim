use crate::{mobject::Mobject, renderer::Renderer};

use super::AnimationFunc;

pub enum Fading {
    Out,
    In,
}

impl<R: Renderer> AnimationFunc<R> for Fading {
    fn pre_anim(&mut self, mobject: &mut Mobject<R>) {
        match self {
            Fading::Out => mobject.set_opacity(1.0),
            Fading::In => mobject.set_opacity(0.0),
        };
    }

    fn interpolate(&mut self, mobject: &mut Mobject<R>, alpha: f32) {
        match self {
            Fading::Out => mobject.set_opacity(1.0 - alpha),
            Fading::In => mobject.set_opacity(alpha),
        };
    }
}
