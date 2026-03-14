use ranim_core::{
    Extract,
    animation::{AnimationCell, Eval},
    core_item::CoreItem,
    traits::Interpolatable,
    utils::rate_functions::smooth,
};

// ANCHOR: MorphRequirement
/// The requirement of [`MorphAnim`]
///
/// Only requires `Extract<Target = CoreItem>`: `T` is converted to `Vec<CoreItem>`
/// first, and interpolation happens at the `Vec<CoreItem>` level.
pub trait MorphRequirement: Clone + Extract<Target = CoreItem> {}
impl<T: Clone + Extract<Target = CoreItem>> MorphRequirement for T {}
// ANCHOR_END: MorphRequirement

// ANCHOR: MorphAnim
/// The methods to create morph animations.
///
/// Morph converts items to `Vec<CoreItem>` via [`Extract`], then interpolates
/// at the core item level. This ensures that all types sharing the same core
/// representation can be morphed uniformly.
pub trait MorphAnim: MorphRequirement + Sized + 'static {
    /// Create a [`Morph`] anim with a func.
    fn morph<F: Fn(&mut Self)>(&mut self, f: F) -> AnimationCell<Vec<CoreItem>>;
    /// Create a [`Morph`] anim from src.
    fn morph_from(&mut self, src: Self) -> AnimationCell<Vec<CoreItem>>;
    /// Create a [`Morph`] anim to dst.
    fn morph_to(&mut self, dst: Self) -> AnimationCell<Vec<CoreItem>>;
}
// ANCHOR_END: MorphAnim

// ANCHOR: MorphAnim-Impl
impl<T: MorphRequirement + 'static> MorphAnim for T {
    fn morph<F: Fn(&mut T)>(&mut self, f: F) -> AnimationCell<Vec<CoreItem>> {
        let mut dst = self.clone();
        (f)(&mut dst);
        let src_items = self.extract();
        let dst_items = dst.extract();
        *self = dst;
        Morph::new(src_items, dst_items)
            .into_animation_cell()
            .with_rate_func(smooth)
    }
    fn morph_from(&mut self, s: T) -> AnimationCell<Vec<CoreItem>> {
        let src_items = s.extract();
        let dst_items = self.extract();
        Morph::new(src_items, dst_items)
            .into_animation_cell()
            .with_rate_func(smooth)
    }
    fn morph_to(&mut self, d: T) -> AnimationCell<Vec<CoreItem>> {
        let src_items = self.extract();
        let dst_items = d.extract();
        *self = d;
        Morph::new(src_items, dst_items)
            .into_animation_cell()
            .with_rate_func(smooth)
    }
}
// ANCHOR_END: MorphAnim-Impl

// ANCHOR: Morph
/// Morph Anim
pub struct Morph<T: Interpolatable> {
    src: T,
    dst: T,
    aligned_src: T,
    aligned_dst: T,
}
// ANCHOR_END: Morph

impl<T: Interpolatable> Morph<T> {
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
impl<T: Interpolatable> Eval<T> for Morph<T> {
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
