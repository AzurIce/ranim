use crate::{rabject::{vmobject::VMobject, RabjectWithId}, scene::Scene};

use super::AnimationFunc;

pub enum Fading {
    Out,
    In,
}

impl AnimationFunc<VMobject> for Fading {
    fn pre_anim(&mut self, rabject: &mut RabjectWithId<VMobject>) {
        match self {
            Fading::Out => rabject.set_opacity(1.0),
            Fading::In => rabject.set_opacity(0.0),
        };
    }

    fn interpolate(&mut self, rabject: &mut RabjectWithId<VMobject>, alpha: f32) {
        match self {
            Fading::Out => rabject.set_opacity(1.0 - alpha),
            Fading::In => rabject.set_opacity(alpha),
        };
    }
}
