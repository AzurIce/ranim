use ranim_core::animation::Eval;

// MARK: LaggedAnim
/// The methods to create animations for `Group<T>`
///
/// # Example
/// ```rust,ignore
/// let item_group: Group::<VItem> = ...;
/// let anim_lagged = item_group.lagged(0.5, |x| x.fade_in()); # lagged with ratio of 0.5
/// let anim_not_lagged = item_group.lagged(0.0, |x| x.fade_in()); # not lagged (anim at the same time)
/// ```
pub trait LaggedAnim<T: Clone>: Sized + 'static {
    /// Create a [`Lagged`] anim.
    fn lagged<E>(&mut self, lag_ratio: f64, anim_func: impl FnMut(&mut T) -> E) -> Lagged<T, E>
    where
        E: Eval<Output = T> + 'static;
}

impl<T: Clone + 'static, I> LaggedAnim<T> for I
where
    for<'a> &'a mut I: IntoIterator<Item = &'a mut T>,
    I: 'static,
{
    fn lagged<E>(&mut self, lag_ratio: f64, anim_func: impl FnMut(&mut T) -> E) -> Lagged<T, E>
    where
        E: Eval<Output = T> + 'static,
    {
        Lagged::new(lag_ratio, self.into_iter().map(anim_func).collect())
    }
}

// pub fn lagged<T, I>(
//     lag_ratio: f64,
//     mut anim_func: impl FnMut(T) -> E,
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
pub struct Lagged<T: Clone, E: Eval<Output = T>> {
    anims: Vec<E>,
    lag_ratio: f64,
    _output: std::marker::PhantomData<fn() -> T>,
}

impl<T: Clone, E: Eval<Output = T>> Lagged<T, E> {
    /// Constructor
    pub fn new(lag_ratio: f64, anims: Vec<E>) -> Self {
        Self {
            anims,
            lag_ratio,
            _output: std::marker::PhantomData,
        }
    }
}

impl<T: Clone, E: Eval<Output = T>> Eval for Lagged<T, E> {
    type Output = Vec<T>;

    fn eval_alpha(&self, alpha: f64) -> Self::Output {
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
