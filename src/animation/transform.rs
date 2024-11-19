use crate::{mobject::Mobject, pipeline::simple};

use super::AnimationFunc;

pub struct Transform {
    target: Mobject<simple::Vertex>,
}

impl Transform {
    pub fn new(target: &Mobject<simple::Vertex>) -> Self {
        Self {
            target: target.clone(),
        }
    }
}

impl AnimationFunc for Transform {
    fn interpolate(&mut self, mobject: &mut Mobject<simple::Vertex>, alpha: f32) {
        if !mobject.aligned_with_mobject(&self.target) {
            mobject.align_with_mobject(&mut self.target);
        }
        mobject.interpolate_with_mobject(&self.target, alpha);
    }
}
