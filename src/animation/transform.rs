use crate::{interpolate::Interpolatable, rabject::Rabject};

use super::{Animation, AnimationFunc};

/// A transform animation func
pub struct Transform<R: Rabject + Alignable + Interpolatable> {
    aligned_source: R,
    aligned_target: R,
}

/// A trait for aligning two rabjects
///
/// Alignment is actually the meaning of preparation for interpolation.
///
/// For example, if we want to interpolate two VMobjects, we need to
/// align their inner data structure `Vec<VMobjectPoint>` to the same length.
pub trait Alignable {
    fn is_aligned(&self, other: &Self) -> bool;
    fn align_with(&mut self, other: &mut Self);
}

impl<R: Rabject + Alignable + Interpolatable + 'static> Transform<R> {
    pub fn new(rabject: R, target: R) -> Animation<R> {
        let mut aligned_source = rabject.clone();
        let mut aligned_target = target.clone();
        if !aligned_source.is_aligned(&aligned_target) {
            aligned_source.align_with(&mut aligned_target);
        }
        // trace!("[Transform::new] aligned_source: {:#?}", aligned_source.points());
        // trace!("[Transform::new] aligned_target: {:#?}", aligned_target.points());

        Animation::new(Self {
            aligned_source,
            aligned_target,
        })
    }
}

impl<R: Rabject + Alignable + Interpolatable> AnimationFunc<R> for Transform<R> {
    fn pre_anim(&mut self, rabject: &mut R) {
        rabject.update_from(&self.aligned_source);
    }

    fn interpolate(&mut self, rabject: &mut R, alpha: f32) {
        let interpolated = self.aligned_source.lerp(&self.aligned_target, alpha);
        rabject.update_from(&interpolated);
    }

    fn post_anim(&mut self, rabject: &mut R) {
        rabject.update_from(&self.aligned_target);
    }
}
