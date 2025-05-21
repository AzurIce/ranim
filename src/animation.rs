//! Evaluation and animation
pub mod creation;
pub mod fading;
pub mod transform;

use crate::utils::rate_functions::linear;

#[allow(unused)]
use log::trace;
use std::{any::Any, fmt::Debug, rc::Rc};

// MARK: Eval
pub trait EvalDynamic<T> {
    fn eval_alpha(&self, alpha: f64) -> T;
}

pub trait ToEvaluator<T> {
    fn to_evaluator(self) -> Evaluator<T>
    where
        Self: Sized + 'static;
}

impl<T: EvalDynamic<E>, E: 'static> ToEvaluator<E> for T {
    fn to_evaluator(self) -> Evaluator<E>
    where
        Self: Sized + 'static,
    {
        Evaluator::new_dynamic(self)
    }
}

pub enum Evaluator<T> {
    Dynamic {
        type_name: String,
        inner: Box<dyn EvalDynamic<T>>,
    },
    Static(Rc<T>),
}

impl<T> Evaluator<T> {
    // Any is for type name
    pub fn new_dynamic<F: EvalDynamic<T> + Any + 'static>(func: F) -> Self {
        let type_name = std::any::type_name::<F>().to_string();
        Self::Dynamic {
            type_name,
            inner: Box::new(func),
        }
    }
}

#[derive(Debug)]
pub enum EvalResult<T> {
    Dynamic(T),
    Static(Rc<T>),
}

impl<T: Clone> EvalResult<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> EvalResult<U> {
        match self {
            Self::Dynamic(t) => EvalResult::Dynamic(f(t)),
            Self::Static(rc) => EvalResult::Static(Rc::new(f((*rc).clone()))),
        }
    }
}

impl<T: Clone> EvalResult<T> {
    pub fn into_owned(self) -> T {
        match self {
            Self::Dynamic(t) => t,
            Self::Static(rc) => (*rc).clone(),
        }
    }
}

pub trait Eval<T> {
    fn eval_alpha(&self, alpha: f64) -> EvalResult<T>;
}

impl<T> Eval<T> for Evaluator<T> {
    fn eval_alpha(&self, alpha: f64) -> EvalResult<T> {
        match self {
            Self::Dynamic {
                inner,
                type_name: _,
            } => EvalResult::Dynamic(inner.eval_alpha(alpha)),
            Self::Static(e) => EvalResult::Static(e.clone()),
        }
    }
}

// MARK: Animation
pub struct AnimationSpan<T> {
    pub(crate) type_name: String,
    pub(crate) evaluator: Box<dyn Eval<T>>,
    pub rate_func: fn(f64) -> f64,
    pub duration_secs: f64,
    pub padding: (f64, f64),
}

impl<T> Debug for AnimationSpan<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Animation {{ duration_secs: {}, padding: {:?}, rate_func: {:?} }}",
            self.duration_secs, self.padding, self.rate_func
        )
    }
}

impl<T: 'static> AnimationSpan<T> {
    pub fn from_evaluator(evaluator: Evaluator<T>) -> Self {
        Self {
            type_name: match &evaluator {
                Evaluator::Dynamic { type_name, .. } => type_name.clone(),
                Evaluator::Static(_) => "Static".to_string(),
            },
            evaluator: Box::new(evaluator),
            rate_func: linear,
            duration_secs: 1.0,
            padding: (0.0, 0.0),
        }
    }
}

impl<T> AnimationSpan<T> {
    pub fn span_len(&self) -> f64 {
        self.duration_secs + self.padding.0 + self.padding.1
    }
    pub fn with_rate_func(mut self, rate_func: fn(f64) -> f64) -> Self {
        self.rate_func = rate_func;
        self
    }
    pub fn with_duration(mut self, secs: f64) -> Self {
        self.duration_secs = secs;
        self
    }
    pub fn padding(mut self, start_sec: f64, end_sec: f64) -> Self {
        self.padding = (start_sec, end_sec);
        self
    }
    pub fn padding_start(mut self, sec: f64) -> Self {
        self.padding.0 = sec;
        self
    }
    pub fn padding_end(mut self, sec: f64) -> Self {
        self.padding.1 = sec;
        self
    }
    pub fn eval_alpha(&self, alpha: f64) -> EvalResult<T> {
        self.eval_sec(alpha * self.span_len())
    }
    pub fn eval_sec(&self, local_sec: f64) -> EvalResult<T> {
        self.evaluator.eval_alpha((self.rate_func)(
            ((local_sec - self.padding.0) / self.duration_secs).clamp(0.0, 1.0),
        ))
    }
}

// MARK: Group
/// A group of animations is basically a type has `iter` and `iter_mut` to iterate
/// over the animations. This trait is automatically implemented.
pub trait AnimGroupFunction<T> {
    /// Get the total duration of the anim group
    fn duration(&self) -> f64;
    /// Sets the rate function for each animation in the group
    fn with_rate_func(self, rate_func: fn(f64) -> f64) -> Self;
    /// Sets the duration for each animation in the group
    fn with_duration(self, secs: f64) -> Self;
    /// Scales the entire group's total duration to a new duration
    ///
    /// For example, use `[x, y, z]`` to represent an anim with duration `y` and padding `(x, z)`,
    /// calling `with_duration(5)` on an group of:
    ///
    /// ```
    ///               [2    , 2    , 2    ]
    ///      [2    , 1, 2    ]
    /// [ 1,  1,  1]
    /// ```
    ///
    /// will scale the group to:
    ///
    /// ```
    ///                [1    , 1    , 1   ]
    ///      [1   , .5, 1    ]
    /// [.5, .5, .5]
    /// ```
    fn with_total_duration(self, secs: f64) -> Self;
    /// Sets the offset of each animation by lagging it
    /// by a given ratio of the previous animation's duration
    fn with_lagged_offset(self, ratio: f64) -> Self;
    /// Sets the epilogue of each animation to the end of the group
    fn with_epilogue_to_end(self) -> Self;
}

impl<E: 'static, T> AnimGroupFunction<E> for T
where
    for<'a> &'a mut T: IntoIterator<Item = &'a mut AnimationSpan<E>>,
    for<'a> &'a T: IntoIterator<Item = &'a AnimationSpan<E>>,
{
    fn duration(&self) -> f64 {
        (&self)
            .into_iter()
            .map(|anim| anim.span_len())
            .reduce(|acc, e| acc.max(e))
            .unwrap()
    }
    fn with_rate_func(mut self, rate_func: fn(f64) -> f64) -> Self {
        (&mut self).into_iter().for_each(|schedule| {
            schedule.rate_func = rate_func;
        });
        self
    }
    fn with_duration(mut self, secs: f64) -> Self {
        (&mut self).into_iter().for_each(|schedule| {
            schedule.duration_secs = secs;
        });
        self
    }
    fn with_total_duration(mut self, secs: f64) -> Self {
        let total_secs = (&self)
            .into_iter()
            .map(|schedule| schedule.span_len())
            .reduce(|acc, e| acc.max(e))
            .unwrap_or(secs);
        let ratio = secs / total_secs;
        (&mut self).into_iter().for_each(|schedule| {
            let (duration, (padding_start, padding_end)) =
                (&mut schedule.duration_secs, &mut schedule.padding);
            *duration *= ratio;
            *padding_start *= ratio;
            *padding_end *= ratio;
        });
        self
    }
    fn with_lagged_offset(mut self, ratio: f64) -> Self {
        let iter = (&self)
            .into_iter()
            .map(|item: &AnimationSpan<E>| item.span_len() * ratio)
            .scan(0.0, |state, x| {
                *state += x;
                Some(*state)
            })
            .collect::<Vec<_>>();

        (&mut self)
            .into_iter()
            .zip([0.0].into_iter().chain(iter))
            .for_each(|(anim, lag_time)| {
                anim.padding.0 = lag_time;
            });

        self
    }
    fn with_epilogue_to_end(mut self) -> Self {
        let duration = self.duration();
        (&mut self).into_iter().for_each(|anim| {
            let span = anim.span_len();
            if span < duration {
                anim.padding.1 = duration - span;
            }
        });
        self
    }
}
