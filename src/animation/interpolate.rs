use super::{AnimScheduler, EntityAnim, PureEvaluator, Rabject};
use crate::{interpolate::Interpolatable, items::Entity};

pub trait InterpolateAnim<'r, 't, T: Entity + Alignable + Interpolatable + 'static> {
    fn interpolate_from(&'r mut self, s: impl Into<T>) -> AnimScheduler<'r, 't, T, EntityAnim<T>>;
}

impl<'r, 't, T: Entity + Alignable + Interpolatable + 'static> InterpolateAnim<'r, 't, T>
    for Rabject<'t, T>
{
    fn interpolate_from(
        &'r mut self,
        src: impl Into<T>,
    ) -> AnimScheduler<'r, 't, T, EntityAnim<T>> {
        let src: T = src.into();
        let func = Interpolate::new(src, self.data.clone());
        AnimScheduler::new(self, EntityAnim::new(self.id, self.data.clone(), func))
    }
}

/// A transform animation func
pub struct Interpolate<T: Entity + Alignable + Interpolatable> {
    src: T,
    dst: T,
    aligned_src: T,
    aligned_dst: T,
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

impl<T: Entity + Alignable + Interpolatable> Interpolate<T> {
    pub fn new(src: T, dst: T) -> Self {
        let mut aligned_src = src.clone();
        let mut aligned_dst = dst.clone();
        if !aligned_src.is_aligned(&aligned_dst) {
            aligned_src.align_with(&mut aligned_dst);
        }
        Self {
            src,
            dst,
            aligned_src,
            aligned_dst,
        }
    }
}

impl<T: Entity + Alignable + Interpolatable> PureEvaluator<T> for Interpolate<T> {
    fn eval_alpha(&self, alpha: f32) -> T {
        if alpha == 0.0 {
            self.src.clone()
        } else if 0.0 < alpha && alpha < 1.0 {
            self.aligned_src.lerp(&self.aligned_dst, alpha)
        } else if alpha == 1.0 {
            self.dst.clone()
        } else {
            unreachable!()
        }
    }
}
