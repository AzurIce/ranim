use crate::{mobject::Mobject, pipeline::PipelineVertex};

use super::AnimationFunc;

pub struct Transform<Vertex: PipelineVertex> {
    target: Mobject<Vertex>,
}

impl<Vertex: PipelineVertex> Transform<Vertex> {
    pub fn new(target: &Mobject<Vertex>) -> Self {
        Self {
            target: target.clone(),
        }
    }
}

impl<Vertex: PipelineVertex> AnimationFunc<Vertex> for Transform<Vertex> {
    fn interpolate(&mut self, mobject: &mut Mobject<Vertex>, alpha: f32) {
        if !mobject.aligned_with_mobject(&self.target) {
            mobject.align_with_mobject(&mut self.target);
        }
        mobject.interpolate_with_mobject(&self.target, alpha);
    }
}
