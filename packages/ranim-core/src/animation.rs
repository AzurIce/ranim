//! Evaluation and animation

use crate::utils::rate_functions::linear;

#[allow(unused)]
use log::trace;
use std::{any::Any, fmt::Debug, sync::Arc};

// MARK: Eval
// ANCHOR: EvalDynamic
/// This is the core of any animation, an animation is basically a function on time.
///
/// This represents a normalized animation function for type `T`, which accepts
/// a progress value `alpha` in range [0, 1] and returns the evaluation result in type `T`.
pub trait EvalDynamic<T> {
    /// Evaluates at the given progress value `alpha` in range [0, 1].
    fn eval_alpha(&self, alpha: f64) -> T;
}
// ANCHOR_END: EvalDynamic

// ANCHOR: Evaluator
/// An Evaluator is whether [`Evaluator::Dynamic`] or [`Evaluator::Static`].
///
/// It has [`Evaluator::eval_alpha`] method which outputs [`EvalResult<T>`].
pub enum Evaluator<T> {
    /// A dynamic evaluator
    Dynamic {
        /// The type name of the evaluator
        type_name: String,
        /// The inner dynamic evaluator
        inner: Box<dyn EvalDynamic<T>>,
    },
    /// A static evaluator
    Static(Arc<T>),
}
// ANCHOR_END: Evaluator

impl<T> Evaluator<T> {
    // Any is for type name
    // TODO: should I include Send here directly?
    /// Creates a dynamic evaluator
    pub fn new_dynamic<F: EvalDynamic<T> + Any + 'static>(func: F) -> Self {
        let type_name = std::any::type_name::<F>().to_string();
        Self::Dynamic {
            type_name,
            inner: Box::new(func),
        }
    }
    /// Creates a static evaluator
    pub fn new_static(e: T) -> Self {
        Self::Static(Arc::new(e))
    }
    /// Evaluates at the given progress value `alpha` in range [0, 1].
    pub fn eval_alpha(&self, alpha: f64) -> EvalResult<T> {
        match self {
            Self::Dynamic {
                inner,
                type_name: _,
            } => EvalResult::Dynamic(inner.eval_alpha(alpha)),
            Self::Static(e) => EvalResult::Static(e.clone()),
        }
    }
}

/// The evaluation result of [`Evaluator`]
#[derive(Debug)]
pub enum EvalResult<T> {
    /// A dynamic evaluation result
    Dynamic(T),
    /// A static evaluation result
    Static(Arc<T>),
}

impl<T: Clone> EvalResult<T> {
    /// Maps the evaluation result to another type
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> EvalResult<U> {
        match self {
            Self::Dynamic(t) => EvalResult::Dynamic(f(t)),
            Self::Static(rc) => EvalResult::Static(Arc::new(f((*rc).clone()))),
        }
    }
}

impl<T: Clone> EvalResult<T> {
    /// Consumes the evaluation result, and convert it into an owned value.
    pub fn into_owned(self) -> T {
        match self {
            Self::Dynamic(t) => t,
            Self::Static(rc) => (*rc).clone(),
        }
    }
}

// MARK: Animation
// ANCHOR: AnimationSpan
/// An [`AnimationSpan<T>`] consist of an [`Evaluator<T>`] and some metadata,
/// such as `rate_func` and `duration_secs`, to control the evaluation process.
pub struct AnimationSpan<T> {
    pub(crate) evaluator: Evaluator<T>,
    /// The rate function used for evaluating
    pub rate_func: fn(f64) -> f64,
    /// The duration seconds
    pub duration_secs: f64,
}
// ANCHOR_END: AnimationSpan

impl<T> Debug for AnimationSpan<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Animation {{ duration_secs: {}, rate_func: {:?} }}",
            self.duration_secs, self.rate_func
        )
    }
}

impl<T: 'static> AnimationSpan<T> {
    /// Construct an [`AnimationSpan`] from [`Evaluator`], this uses [`linear`]
    /// rate function and `1.0` duration seconds for default.
    pub fn from_evaluator(evaluator: Evaluator<T>) -> Self {
        Self {
            evaluator,
            rate_func: linear,
            duration_secs: 1.0,
        }
    }
}

impl<T> AnimationSpan<T> {
    /// Get the type name of the [`Evaluator`] of this [`AnimationSpan`].
    pub fn type_name(&self) -> &str {
        match &self.evaluator {
            Evaluator::Dynamic { type_name, .. } => type_name,
            Evaluator::Static(_) => "Static",
        }
    }
    /// A builder func to modify `rate_func`
    pub fn with_rate_func(mut self, rate_func: fn(f64) -> f64) -> Self {
        self.rate_func = rate_func;
        self
    }
    /// A builder func to modify `secs`
    pub fn with_duration(mut self, secs: f64) -> Self {
        self.duration_secs = secs;
        self
    }
}

// ANCHOR: AnimationSpan-eval
impl<T> AnimationSpan<T> {
    /// Evaluate at the given progress value `alpha` in [0, 1]
    pub fn eval_alpha(&self, alpha: f64) -> EvalResult<T> {
        self.eval_sec(alpha * self.duration_secs)
    }
    /// Evaluate at the given second `sec`
    pub fn eval_sec(&self, sec: f64) -> EvalResult<T> {
        self.evaluator
            .eval_alpha((self.rate_func)((sec / self.duration_secs).clamp(0.0, 1.0)))
    }
}
// ANCHOR_END: AnimationSpan-eval
