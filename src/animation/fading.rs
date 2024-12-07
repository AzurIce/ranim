use crate::rabject::{vmobject::VMobject, Rabject};

use super::{Animation, AnimationConfig, AnimationFunc};

pub enum Fading {
    Out,
    In,
}

impl Fading {
    pub fn fade_in() -> Animation<VMobject> {
        Animation::new(Self::In)
    }

    pub fn fade_out() -> Animation<VMobject> {
        Animation::new(Self::Out)
    }
}

impl AnimationFunc<VMobject> for Fading {
    fn pre_anim(&mut self, rabject: &mut VMobject) {
        match self {
            Fading::Out => rabject.set_opacity(1.0),
            Fading::In => rabject.set_opacity(0.0),
        };
    }

    fn interpolate(&mut self, rabject: &mut VMobject, alpha: f32) {
        match self {
            Fading::Out => rabject.set_opacity(1.0 - alpha),
            Fading::In => rabject.set_opacity(alpha),
        };
    }
}
