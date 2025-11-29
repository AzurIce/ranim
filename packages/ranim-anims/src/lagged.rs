use ranim_core::animation::{AnimationCell, Eval};

// MARK: LaggedAnim
/// The methods to create animations for `Group<T>`
///
/// # Example
/// ```rust,ignore
/// let item_group: Group::<VItem> = ...;
/// let anim_lagged = item_group.lagged(0.5, |x| x.fade_in()); # lagged with ratio of 0.5
/// let anim_not_lagged = item_group.lagged(0.0, |x| x.fade_in()); # not lagged (anim at the same time)
/// ```
pub trait LaggedAnim<T>: Sized + 'static {
    /// Create a [`Lagged`] anim.
    fn lagged(
        &mut self,
        lag_ratio: f64,
        anim_func: impl FnMut(&mut T) -> AnimationCell<T>,
    ) -> AnimationCell<Vec<T>>;
}

impl<T: Clone + 'static, I> LaggedAnim<T> for I
where
    for<'a> &'a mut I: IntoIterator<Item = &'a mut T>,
    I: 'static,
{
    fn lagged(
        &mut self,
        lag_ratio: f64,
        anim_func: impl FnMut(&mut T) -> AnimationCell<T>,
    ) -> AnimationCell<Vec<T>> {
        let anim =
            Lagged::new(lag_ratio, self.into_iter().map(anim_func).collect()).into_animation_cell();
        anim
    }
}

// pub fn lagged<T, I>(
//     lag_ratio: f64,
//     mut anim_func: impl FnMut(T) -> AnimationCell<T>,
// ) -> impl FnMut(I) -> Lagged<T>
// where
//     I: IntoIterator<Item = T>,
// {
//     move |target| Lagged::new(target, lag_ratio, &mut anim_func)
// }

/// The lagged anim.
///
/// This is applyable to `IntoIterator<Item = T>`, and this will apply
/// the anims in the order of the elements with the lag ratio.
pub struct Lagged<T> {
    anims: Vec<AnimationCell<T>>,
    lag_ratio: f64,
}

impl<T> Lagged<T> {
    /// Constructor
    pub fn new(lag_ratio: f64, anims: Vec<AnimationCell<T>>) -> Self {
        Self { anims, lag_ratio }
    }
}

impl<T: Clone, I: FromIterator<T>> Eval<I> for Lagged<T> {
    fn eval_alpha(&self, alpha: f64) -> I {
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
                anim.eval_alpha(alpha)
            })
            .collect()
    }
}
