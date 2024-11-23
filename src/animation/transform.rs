use crate::{mobject::Mobject, renderer::Renderer};

use super::AnimationFunc;

pub struct Transform<R: Renderer> {
    target: Mobject<R>,
}

impl<R: Renderer> Transform<R> {
    pub fn new(target: &Mobject<R>) -> Self {
        Self {
            target: target.clone(),
        }
    }
}

impl<R: Renderer> AnimationFunc<R> for Transform<R> {
    fn interpolate(&mut self, mobject: &mut Mobject<R>, alpha: f32) {
        if !mobject.aligned_with_mobject(&self.target) {
            mobject.align_with_mobject(&mut self.target);
        }
        mobject.interpolate_with_mobject(&self.target, alpha);
    }
}
