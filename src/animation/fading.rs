use crate::{mobject::Mobject, pipeline::PipelineVertex};

use super::AnimationFunc;

pub enum Fading {
    Out,
    In,
}

impl<Vertex: PipelineVertex> AnimationFunc<Vertex> for Fading {
    fn pre_anim(&mut self, mobject: &mut Mobject<Vertex>) {
        match self {
            Fading::Out => mobject.set_opacity(1.0),
            Fading::In => mobject.set_opacity(0.0),
        };
    }

    fn interpolate(&mut self, mobject: &mut Mobject<Vertex>, alpha: f32) {
        match self {
            Fading::Out => mobject.set_opacity(1.0 - alpha),
            Fading::In => mobject.set_opacity(alpha),
        };
    }
}
