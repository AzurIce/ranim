use ranim_core::{
    animation::{AnimationCell, Eval},
    traits::{Alignable, Interpolatable},
    utils::rate_functions::smooth,
};

// ANCHOR: TransformRequirement
/// The requirement of [`Transform`]
pub trait TransformRequirement: Alignable + Interpolatable + Clone {}
impl<T: Alignable + Interpolatable + Clone> TransformRequirement for T {}
// ANCHOR_END: TransformRequirement

/// The methods to create animations for `T` that satisfies [`TransformRequirement`]
pub trait TransformAnim: TransformRequirement + Sized + 'static {
    /// Create a [`Transform`] anim with a func.
    fn transform<F: Fn(&mut Self)>(self, f: F) -> AnimationCell<Self>;

    fn transform_ref<F: Fn(&mut Self)>(&self, f: F) -> AnimationCell<Self> {
        self.clone().transform(f)
    }
    fn transform_mut<F: Fn(&mut Self)>(&mut self, f: F) -> AnimationCell<Self> {
        let anim = self.transform_ref(f);
        *self = anim.eval_alpha(1.0);
        anim
    }
    /// Create a [`Transform`] anim from src.
    fn transform_from(self, src: Self) -> AnimationCell<Self>;
    fn transform_from_ref(&self, src: Self) -> AnimationCell<Self> {
        self.clone().transform_from(src)
    }
    fn transform_from_mut(&mut self, src: Self) -> AnimationCell<Self> {
        let anim = self.transform_from_ref(src);
        *self = anim.eval_alpha(1.0);
        anim
    }
    /// Create a [`Transform`] anim to dst.
    fn transform_to(self, dst: Self) -> AnimationCell<Self>;
    fn transform_to_ref(&self, dst: Self) -> AnimationCell<Self> {
        self.clone().transform_to(dst)
    }
    fn transform_to_mut(&mut self, dst: Self) -> AnimationCell<Self> {
        let anim = self.transform_to_ref(dst);
        *self = anim.eval_alpha(1.0);
        anim
    }
}

impl<T: TransformRequirement + 'static> TransformAnim for T {
    fn transform<F: Fn(&mut T)>(self, f: F) -> AnimationCell<T> {
        let mut dst = self.clone();
        (f)(&mut dst);
        Transform::new(self.clone(), dst)
            .into_animation_cell()
            .with_rate_func(smooth)
    }
    fn transform_from(self, s: T) -> AnimationCell<T> {
        Transform::new(s, self.clone())
            .into_animation_cell()
            .with_rate_func(smooth)
    }
    fn transform_to(self, d: T) -> AnimationCell<T> {
        Transform::new(self.clone(), d)
            .into_animation_cell()
            .with_rate_func(smooth)
    }
}

// ANCHOR: Transform
/// Transform Anim
pub struct Transform<T: TransformRequirement> {
    src: T,
    dst: T,
    aligned_src: T,
    aligned_dst: T,
}
// ANCHOR_END: Transform

impl<T: TransformRequirement> Transform<T> {
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

// ANCHOR: Transform-EvalDynamic
impl<T: TransformRequirement> Eval<T> for Transform<T> {
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
// ANCHOR_END: Transform-EvalDynamic
