//! Evaluation and animation
pub mod creation;
pub mod fading;
pub mod func;
pub mod lagged;
pub mod transform;

use crate::utils::rate_functions::linear;

#[allow(unused)]
use log::trace;
use std::{any::Any, fmt::Debug, sync::Arc};

// MARK: Eval
// ANCHOR: EvalDynamic
pub trait EvalDynamic<T> {
    fn eval_alpha(&self, alpha: f64) -> T;
}
// ANCHOR_END: EvalDynamic

// ANCHOR: Evaluator
pub enum Evaluator<T> {
    Dynamic {
        type_name: String,
        inner: Box<dyn EvalDynamic<T>>,
    },
    Static(Arc<T>),
}
// ANCHOR_END: Evaluator

impl<T> Evaluator<T> {
    // Any is for type name
    // TODO: should I include Send here directly?
    pub fn new_dynamic<F: EvalDynamic<T> + Any + 'static>(func: F) -> Self {
        let type_name = std::any::type_name::<F>().to_string();
        Self::Dynamic {
            type_name,
            inner: Box::new(func),
        }
    }
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

#[derive(Debug)]
pub enum EvalResult<T> {
    Dynamic(T),
    Static(Arc<T>),
}

impl<T: Clone> EvalResult<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> EvalResult<U> {
        match self {
            Self::Dynamic(t) => EvalResult::Dynamic(f(t)),
            Self::Static(rc) => EvalResult::Static(Arc::new(f((*rc).clone()))),
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

// MARK: Animation
// ANCHOR: AnimationSpan
pub struct AnimationSpan<T> {
    pub(crate) evaluator: Evaluator<T>,
    pub rate_func: fn(f64) -> f64,
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
    pub fn from_evaluator(evaluator: Evaluator<T>) -> Self {
        Self {
            evaluator,
            rate_func: linear,
            duration_secs: 1.0,
        }
    }
}

impl<T> AnimationSpan<T> {
    pub fn type_name(&self) -> &str {
        match &self.evaluator {
            Evaluator::Dynamic { type_name, .. } => type_name,
            Evaluator::Static(_) => "Static",
        }
    }
    pub fn with_rate_func(mut self, rate_func: fn(f64) -> f64) -> Self {
        self.rate_func = rate_func;
        self
    }
    pub fn with_duration(mut self, secs: f64) -> Self {
        self.duration_secs = secs;
        self
    }
}

// ANCHOR: AnimationSpan-eval
impl<T> AnimationSpan<T> {
    pub fn eval_alpha(&self, alpha: f64) -> EvalResult<T> {
        self.eval_sec(alpha * self.duration_secs)
    }
    pub fn eval_sec(&self, local_sec: f64) -> EvalResult<T> {
        self.evaluator.eval_alpha((self.rate_func)(
            (local_sec / self.duration_secs).clamp(0.0, 1.0),
        ))
    }
}
// ANCHOR_END: AnimationSpan-eval
