use ranim_core::{
    animation::eval::{Eval, EvalExt},
    traits::{Alignable, Interpolatable},
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
    fn morph<F: Fn(&mut Self)>(&mut self, f: F) -> Morph<Self>;
    /// Create a [`Morph`] anim from src.
    fn morph_from(&mut self, src: Self) -> Morph<Self>;
    /// Create a [`Morph`] anim to dst.
    fn morph_to(&mut self, dst: Self) -> Morph<Self>;
}
// ANCHOR_END: MorphAnim

// ANCHOR: MorphAnim-Impl
impl<T: MorphRequirement + 'static> MorphAnim for T {
    fn morph<F: Fn(&mut T)>(&mut self, f: F) -> Morph<T> {
        let mut dst = self.clone();
        (f)(&mut dst);
        Morph::new(self.clone(), dst).apply_to(self)
    }
    fn morph_from(&mut self, s: T) -> Morph<T> {
        Morph::new(s, self.clone()).apply_to(self)
    }
    fn morph_to(&mut self, d: T) -> Morph<T> {
        Morph::new(self.clone(), d).apply_to(self)
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
impl<T: MorphRequirement> Eval for Morph<T> {
    type Output = T;

    fn eval_alpha(&self, alpha: f64) -> Self::Output {
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
