use ranim_core::{
    Group,
    animation::{AnimationSpan, EvalDynamic, Evaluator},
    traits::Opacity,
};

// MARK: LaggedAnim
/// The methods to create animations for `Group<T>`
pub trait LaggedAnim<T> {
    /// Create a [`Lagged`] anim.
    fn lagged(
        self,
        lag_ratio: f64,
        anim_func: impl FnMut(T) -> AnimationSpan<T>,
    ) -> AnimationSpan<Self>
    where
        Self: Sized;
}

impl<T: Clone + 'static> LaggedAnim<T> for Group<T> {
    fn lagged(
        self,
        lag_ratio: f64,
        anim_func: impl FnMut(T) -> AnimationSpan<T>,
    ) -> AnimationSpan<Self> {
        AnimationSpan::from_evaluator(Evaluator::new_dynamic(Lagged::new(
            self, lag_ratio, anim_func,
        )))
    }
}

// pub fn lagged<T, I>(
//     lag_ratio: f64,
//     mut anim_func: impl FnMut(T) -> AnimationSpan<T>,
// ) -> impl FnMut(I) -> Lagged<T>
// where
//     I: IntoIterator<Item = T>,
// {
//     move |target| Lagged::new(target, lag_ratio, &mut anim_func)
// }

/// The lagged anim.
///
/// This is only applyable to [`Group<T>`], and this will apply
/// the anims in the order of the elements with the lag ratio.
pub struct Lagged<T> {
    anims: Vec<AnimationSpan<T>>,
    lag_ratio: f64,
}

impl<T> Lagged<T> {
    /// Constructor
    pub fn new<I>(target: I, lag_ratio: f64, anim_func: impl FnMut(T) -> AnimationSpan<T>) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            anims: target.into_iter().map(anim_func).collect(),
            lag_ratio,
        }
    }
}

impl<T: Clone> EvalDynamic<Group<T>> for Lagged<T> {
    fn eval_alpha(&self, alpha: f64) -> Group<T> {
        // -|--
        //  -|--
        //   -|--
        // total_time - unit_time * (1.0 - lag_ratio)  = unit_time * lag_ratio * n
        // total_time = unit_time * (1.0 + (n - 1) lag_ratio)
        let unit_time = 1.0 / (1.0 + (self.anims.len() - 1) as f64 * self.lag_ratio);
        let unit_lagged_time = unit_time * self.lag_ratio;
        self.anims
            .iter()
            .enumerate()
            .map(|(i, anim)| {
                let start = unit_lagged_time * i as f64;

                let alpha = (alpha - start) / unit_time;
                let alpha = alpha.clamp(0.0, 1.0);
                anim.eval_alpha(alpha).into_owned()
            })
            .collect()
    }
}
