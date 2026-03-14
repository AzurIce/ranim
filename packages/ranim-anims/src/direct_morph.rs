//! Direct morph animation — interpolates directly on `T` without converting to core items.

use ranim_core::{
    animation::{AnimationCell, Eval},
    traits::Interpolatable,
    utils::rate_functions::smooth,
};

use crate::morph::Morph;

/// The requirement for [`DirectMorphAnim`]
pub trait DirectMorphRequirement: Interpolatable {}
impl<T: Interpolatable> DirectMorphRequirement for T {}

/// The methods to create direct morph animations for `T`.
///
/// Unlike [`super::morph::MorphAnim`], this performs alignment and interpolation directly
/// on the `T` type without converting to core items first.
pub trait DirectMorphAnim: DirectMorphRequirement + Sized + 'static {
    /// Create a direct morph anim with a func.
    fn direct_morph<F: Fn(&mut Self)>(&mut self, f: F) -> AnimationCell<Self>;
    /// Create a direct morph anim from src.
    fn direct_morph_from(&mut self, src: Self) -> AnimationCell<Self>;
    /// Create a direct morph anim to dst.
    fn direct_morph_to(&mut self, dst: Self) -> AnimationCell<Self>;
}

impl<T: DirectMorphRequirement + 'static> DirectMorphAnim for T {
    fn direct_morph<F: Fn(&mut T)>(&mut self, f: F) -> AnimationCell<T> {
        let mut dst = self.clone();
        (f)(&mut dst);
        Morph::new(self.clone(), dst)
            .into_animation_cell()
            .with_rate_func(smooth)
            .apply_to(self)
    }
    fn direct_morph_from(&mut self, s: T) -> AnimationCell<T> {
        Morph::new(s, self.clone())
            .into_animation_cell()
            .with_rate_func(smooth)
            .apply_to(self)
    }
    fn direct_morph_to(&mut self, d: T) -> AnimationCell<T> {
        Morph::new(self.clone(), d)
            .into_animation_cell()
            .with_rate_func(smooth)
            .apply_to(self)
    }
}
