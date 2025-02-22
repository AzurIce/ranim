use super::{AnimWithParams, EntityAnim, PureEvaluator, Rabject};
use crate::{
    interpolate::Interpolatable,
    items::{ConvertIntoRabject, Entity},
};

pub fn interpolate<D: Entity + Alignable + Interpolatable + 'static, S: ConvertIntoRabject<D>>(
    src: &S,
    dst: &Rabject<D>,
) -> AnimWithParams<EntityAnim<D>> {
    let src: Rabject<D> = src.clone().convert_into();
    let src_data = src.data.clone();
    AnimWithParams::new(EntityAnim::new(
        src.clone(),
        Interpolate::new(src_data, dst.data.clone()),
    ))
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
