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
    fn fade_in(&mut self) -> AnimationCell<Self>;
    /// Create a [`FadeOut`] anim.
    fn fade_out(&mut self) -> AnimationCell<Self>;
}

impl<T: FadingRequirement + Sized + 'static> FadingAnim for T {
    fn fade_in(&mut self) -> AnimationCell<Self> {
        FadeIn::new(self.clone())
            .into_animation_cell()
            .with_rate_func(smooth)
            .apply_to(self)
    }
    fn fade_out(&mut self) -> AnimationCell<Self> {
        FadeOut::new(self.clone())
            .into_animation_cell()
            .with_rate_func(smooth)
            .apply_to(self)
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
