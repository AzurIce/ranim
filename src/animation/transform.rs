use crate::{mobject::Mobject, renderer::RendererVertex};

use super::AnimationFunc;

pub struct Transform<Vertex: RendererVertex> {
    target: Mobject<Vertex>,
}

impl<Vertex: RendererVertex> Transform<Vertex> {
    pub fn new(target: &Mobject<Vertex>) -> Self {
        Self {
            target: target.clone(),
        }
    }
}

impl<Vertex: RendererVertex> AnimationFunc<Vertex> for Transform<Vertex> {
    fn interpolate(&mut self, mobject: &mut Mobject<Vertex>, alpha: f32) {
        if !mobject.aligned_with_mobject(&self.target) {
            mobject.align_with_mobject(&mut self.target);
        }
        mobject.interpolate_with_mobject(&self.target, alpha);
    }
}
