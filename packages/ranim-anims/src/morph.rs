use ranim_core::{
    animation::{AnimationCell, Eval},
    traits::{Alignable, Interpolatable},
    utils::rate_functions::smooth,
};

// ANCHOR: MorphRequirement
/// The requirement of [`Morph`]
pub trait MorphRequirement: Alignable + Interpolatable + Clone {}
impl<T: Alignable + Interpolatable + Clone> MorphRequirement for T {}
// ANCHOR_END: MorphRequirement

// ANCHOR: MorphAnim
/// The methods to create animations for `T` that satisfies [`MorphRequirement`]
pub trait MorphAnim: MorphRequirement + Sized + 'static {
    /// Create a [`Morph`] anim with a func.
    fn morph<F: Fn(&mut Self)>(&mut self, f: F) -> AnimationCell<Self>;
    /// Create a [`Morph`] anim from src.
    fn morph_from(&mut self, src: Self) -> AnimationCell<Self>;
    /// Create a [`Morph`] anim to dst.
    fn morph_to(&mut self, dst: Self) -> AnimationCell<Self>;
}
// ANCHOR_END: MorphAnim

// ANCHOR: MorphAnim-Impl
impl<T: MorphRequirement + 'static> MorphAnim for T {
    fn morph<F: Fn(&mut T)>(&mut self, f: F) -> AnimationCell<T> {
        let mut dst = self.clone();
        (f)(&mut dst);
        Morph::new(self.clone(), dst)
            .into_animation_cell()
            .with_rate_func(smooth)
            .apply_to(self)
    }
    fn morph_from(&mut self, s: T) -> AnimationCell<T> {
        Morph::new(s, self.clone())
            .into_animation_cell()
            .with_rate_func(smooth)
            .apply_to(self)
    }
    fn morph_to(&mut self, d: T) -> AnimationCell<T> {
        Morph::new(self.clone(), d)
            .into_animation_cell()
            .with_rate_func(smooth)
            .apply_to(self)
    }
}
// ANCHOR_END: MorphAnim-Impl

// ANCHOR: Morph
/// Morph Anim
pub struct Morph<T: MorphRequirement> {
    src: T,
    dst: T,
    aligned_src: T,
    aligned_dst: T,
}
// ANCHOR_END: Morph

impl<T: MorphRequirement> Morph<T> {
    /// Constructor
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

// ANCHOR: Morph-Eval
impl<T: MorphRequirement> Eval<T> for Morph<T> {
    fn eval_alpha(&self, alpha: f64) -> T {
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
// ANCHOR_END: Morph-Eval
