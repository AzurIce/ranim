use ranim_core::animation::{AnimationCell, Eval};

// MARK: Require Trait
/// The requirement for [`Func`]
pub trait FuncRequirement: Clone {}
impl<T: Clone> FuncRequirement for T {}

// MARK: Anim Trait
/// The methods to create animations for `T` that satisfies [`FuncRequirement`]
pub trait FuncAnim: FuncRequirement + 'static {
    /// Create a [`Func`] anim.
    fn func(&mut self, f: impl Fn(&Self, f64) -> Self + 'static) -> AnimationCell<Self>;
}

impl<T: FuncRequirement + 'static> FuncAnim for T {
    fn func(&mut self, f: impl Fn(&Self, f64) -> Self + 'static) -> AnimationCell<Self> {
        Func::new(self.clone(), f)
            .into_animation_cell()
            .apply_to(self)
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

impl<T: FuncRequirement> Eval<T> for Func<T> {
    fn eval_alpha(&self, alpha: f64) -> T {
        (self.f)(&self.src, alpha)
    }
}
