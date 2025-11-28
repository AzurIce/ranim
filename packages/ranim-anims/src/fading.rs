use ranim_core::{
    animation::{AnimationCell, Eval},
    traits::{Interpolatable, Opacity},
    utils::rate_functions::smooth,
};

// MARK: Require Trait
/// The requirement of [`FadeIn`] and [`FadeOut`]
pub trait FadingRequirement: Opacity + Interpolatable + Clone {}
impl<T: Opacity + Interpolatable + Clone> FadingRequirement for T {}

// MARK: Anim Trait
/// The methods to create animations for `T` that satisfies [`FadingRequirement`]
pub trait FadingAnim: FadingRequirement + Sized + 'static {
    /// Create a [`FadeIn`] anim.
    fn fade_in(self) -> AnimationCell<Self>;
    fn fade_in_ref(&self) -> AnimationCell<Self> {
        self.clone().fade_in()
    }
    fn fade_in_mut(&mut self) -> AnimationCell<Self> {
        let anim = self.fade_in_ref();
        *self = anim.eval_alpha(1.0);
        anim
    }
    /// Create a [`FadeOut`] anim.
    fn fade_out(self) -> AnimationCell<Self>;
    fn fade_out_ref(&self) -> AnimationCell<Self> {
        self.clone().fade_out()
    }
    fn fade_out_mut(&mut self) -> AnimationCell<Self> {
        let anim = self.fade_out_ref();
        *self = anim.eval_alpha(1.0);
        anim
    }
}

impl<T: FadingRequirement + Sized + 'static> FadingAnim for T {
    fn fade_in(self) -> AnimationCell<Self> {
        FadeIn::new(self.clone())
            .into_animation_cell()
            .with_rate_func(smooth)
    }
    fn fade_out(self) -> AnimationCell<Self> {
        FadeOut::new(self.clone())
            .into_animation_cell()
            .with_rate_func(smooth)
    }
}

// MARK: Impl

/// Fade-in animation.
///
/// Because some Items may not be completly opaque, so
/// this is implemented by setting the opacity to 0.0 as
/// initial state, then interpolate between them.
pub struct FadeIn<T: FadingRequirement> {
    src: T,
    dst: T,
}

impl<T: FadingRequirement> FadeIn<T> {
    /// Constructor
    pub fn new(target: T) -> Self {
        let mut src = target.clone();
        let dst = target.clone();
        src.set_opacity(0.0);
        Self { src, dst }
    }
}

impl<T: FadingRequirement> Eval<T> for FadeIn<T> {
    fn eval_alpha(&self, alpha: f64) -> T {
        self.src.lerp(&self.dst, alpha)
    }
}

/// Fade-out animation.
///
/// Because some Items may not be completly opaque, so
/// this is implemented by setting the opacity to 0.0 as
/// target state, then interpolate between them.
pub struct FadeOut<T: FadingRequirement> {
    src: T,
    dst: T,
}

impl<T: FadingRequirement> FadeOut<T> {
    /// Constructor
    pub fn new(target: T) -> Self {
        let src = target.clone();
        let mut dst = target.clone();
        dst.set_opacity(0.0);
        Self { src, dst }
    }
}

impl<T: FadingRequirement> Eval<T> for FadeOut<T> {
    fn eval_alpha(&self, alpha: f64) -> T {
        self.src.lerp(&self.dst, alpha)
    }
}
