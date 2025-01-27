use crate::interpolate::Interpolatable;

use crate::items::Updatable;

use super::{Animation, AnimationFunc};

/// A transform animation func
pub struct Interpolate<T: Alignable + Interpolatable> {
    aligned_source: Option<T>,
    aligned_target: T,
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

impl<T: Alignable + Interpolatable + Clone + 'static> Interpolate<T> {
    pub fn new(target: T) -> Animation<T> {
        let aligned_target = target.into();

        Animation::new(Self {
            aligned_source: None,
            aligned_target,
        })
    }
}

impl<T: Alignable + Interpolatable + Clone> AnimationFunc<T> for Interpolate<T> {
    fn init(&mut self, entity: &mut T) {
        let mut aligned_source = entity.clone();
        if !aligned_source.is_aligned(&self.aligned_target) {
            aligned_source.align_with(&mut self.aligned_target);
        }
        self.aligned_source = Some(aligned_source);

        entity.update_from(self.aligned_source.as_ref().unwrap());
    }

    fn interpolate(&mut self, entity: &mut T, alpha: f32) {
        let interpolated = self
            .aligned_source
            .as_ref()
            .unwrap()
            .lerp(&self.aligned_target, alpha);
        entity.update_from(&interpolated);
    }

    fn post_anim(&mut self, entity: &mut T) {
        entity.update_from(&self.aligned_target);
    }
}
