use ranim_core::animation::{AnimationSpan, EvalDynamic};

// MARK: Require Trait
/// The requirement for [`Func`]
pub trait FuncRequirement: Clone {}

// MARK: Anim Trait
/// The methods to create animations for `T` that satisfies [`FuncRequirement`]
pub trait FuncAnim<T: FuncRequirement + 'static> {
    /// Create a [`Func`] anim.
    fn func(self, f: impl Fn(&T, f64) -> T + 'static) -> AnimationSpan<T>;
}

impl<T: FuncRequirement + 'static> FuncAnim<T> for T {
    fn func(self, f: impl Fn(&T, f64) -> T + 'static) -> AnimationSpan<T> {
        Func::new(self.clone(), f).into_animation_span()
    }
}

// MARK: Impl
/// An func anim.
///
/// This simply use the given func to eval the animation state.
pub struct Func<T: FuncRequirement> {
    src: T,
    #[allow(clippy::type_complexity)]
    f: Box<dyn Fn(&T, f64) -> T>,
}

impl<T: FuncRequirement> Func<T> {
    /// Constructor
    pub fn new(target: T, f: impl Fn(&T, f64) -> T + 'static) -> Self {
        Self {
            src: target,
            f: Box::new(f),
        }
    }
}

impl<T: FuncRequirement> EvalDynamic<T> for Func<T> {
    fn eval_alpha(&self, alpha: f64) -> T {
        (self.f)(&self.src, alpha)
    }
}
