//! Evaluation and animation
pub mod creation;
pub mod fading;
pub mod transform;

use crate::{
    items::{Rabject, group::Group},
    timeline::RanimItem,
    utils::rate_functions::linear,
};

#[allow(unused)]
use log::trace;
use std::{any::Any, fmt::Debug, iter::Once, rc::Rc};

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

    // // Any is for type name
    // pub fn from_eval_dynamic<F: EvalDynamic<T> + Any + 'static>(func: F) -> Self {
    //     let type_name = std::any::type_name::<F>().to_string();
    //     Self::Dynamic {
    //         type_name,
    //         inner: Box::new(func),
    //     }
    // }
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

pub struct EvalAdapter<T> {
    pub inner: Box<dyn Eval<T>>,
}

// MARK: Animation
pub struct AnimationSpan<T> {
    pub(crate) type_name: String,
    pub(crate) evaluator: Box<dyn Eval<T>>,
    pub(crate) rate_func: fn(f64) -> f64,
    pub(crate) duration_secs: f64,
    pub(crate) padding: (f64, f64),
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

// MARK: AnimSchedule

/// A schedule for an animation
///
/// When you create an anim, you actually creates an [`AnimSchedule`] which contains the anim.
/// The rabject's data won't change unless you call [`AnimSchedule::apply`].
pub struct AnimSchedule<'r, T> {
    pub(crate) rabject: &'r mut Rabject<T>,
    pub(crate) anim: AnimationSpan<T>,
}

impl<T> Debug for AnimSchedule<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AnimSchedule {{ rabject: {:?}, anim: {:?} }}",
            self.rabject.id, self.anim
        )
    }
}

impl<'r, T: 'static> AnimSchedule<'r, T> {
    pub fn new(rabject: &'r mut Rabject<T>, anim: AnimationSpan<T>) -> Self {
        Self { rabject, anim }
    }
}

impl<T> AnimSchedule<'_, T> {
    pub fn with_padding(mut self, start_sec: f64, end_sec: f64) -> Self {
        self.anim.padding = (start_sec, end_sec);
        self
    }
    pub fn with_rate_func(mut self, rate_func: fn(f64) -> f64) -> Self {
        self.anim.rate_func = rate_func;
        self
    }
    pub fn with_duration(mut self, secs: f64) -> Self {
        self.anim.duration_secs = secs;
        self
    }
}

impl<T: 'static> IntoIterator for AnimSchedule<'_, T> {
    type IntoIter = Once<Self>;
    type Item = Self;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}

// MARK: Group

impl<T: 'static> Group<AnimSchedule<'_, T>> {
    /// Sets the rate function for each animation in the group
    pub fn with_rate_func(mut self, rate_func: fn(f64) -> f64) -> Self {
        self.iter_mut().for_each(|schedule| {
            schedule.anim.rate_func = rate_func;
        });
        self
    }
    /// Sets the duration for each animation in the group
    pub fn with_duration(mut self, secs: f64) -> Self {
        self.iter_mut().for_each(|schedule| {
            schedule.anim.duration_secs = secs;
        });
        self
    }
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
    pub fn with_total_duration(mut self, secs: f64) -> Self {
        let total_secs = self
            .iter()
            .map(|schedule| schedule.anim.span_len())
            .reduce(|acc, e| acc.max(e))
            .unwrap_or(secs);
        let ratio = secs / total_secs;
        self.iter_mut().for_each(|schedule| {
            let (duration, (padding_start, padding_end)) =
                (&mut schedule.anim.duration_secs, &mut schedule.anim.padding);
            *duration *= ratio;
            *padding_start *= ratio;
            *padding_end *= ratio;
        });
        self
    }
}

impl<T: RanimItem + Clone + 'static> AnimSchedule<'_, T> {
    pub fn apply(self) -> Self {
        if let EvalResult::Dynamic(res) = self.anim.eval_alpha(1.0) {
            self.rabject.data = res;
        }
        self
    }
}

impl<T: RanimItem + Clone + 'static> Group<AnimSchedule<'_, T>> {
    pub fn apply(mut self) -> Self {
        self.iter_mut().for_each(|schedule| {
            if let EvalResult::Dynamic(res) = schedule.anim.eval_alpha(1.0) {
                schedule.rabject.data = res;
            }
        });
        self
    }
}
