//! Evaluation and animation
pub mod creation;
pub mod fading;
pub mod transform;

use crate::{items::Rabject, timeline::EntityTimelineStaticState, utils::rate_functions::linear};

#[allow(unused)]
use log::trace;
use std::rc::Rc;

// MARK: Eval

pub trait EvalDynamic<T> {
    fn eval_alpha(&self, alpha: f32) -> T;
}

pub trait Schedule<T> {
    fn schedule<'r, 't>(self, target: &'r mut Rabject<'t, T>) -> AnimSchedule<'r, 't, T>
    where
        Self: Sized + 'static;
}

impl<T: EvalDynamic<E>, E: 'static> Schedule<E> for T {
    fn schedule<'r, 't>(self, target: &'r mut Rabject<'t, E>) -> AnimSchedule<'r, 't, E>
    where
        Self: Sized + 'static,
    {
        AnimSchedule::new(target, Evaluator::Dynamic(Box::new(self)))
    }
}

pub enum Evaluator<T> {
    Dynamic(Box<dyn EvalDynamic<T>>),
    Static(Rc<T>),
}

impl<T> From<Box<dyn EvalDynamic<T>>> for Evaluator<T> {
    fn from(value: Box<dyn EvalDynamic<T>>) -> Self {
        Self::Dynamic(value)
    }
}

impl<T: EntityTimelineStaticState + Clone + 'static> EvalDynamic<T::StateType>
    for Box<dyn EvalDynamic<T>>
{
    fn eval_alpha(&self, alpha: f32) -> T::StateType {
        self.as_ref().eval_alpha(alpha).into_state_type()
    }
}

impl<T> Evaluator<T> {
    pub fn new_dynamic<F: EvalDynamic<T> + 'static>(func: F) -> Self {
        Self::Dynamic(Box::new(func))
    }
}

impl<T: EntityTimelineStaticState + Clone + 'static> Evaluator<T> {
    pub fn to_state_type(self) -> Evaluator<T::StateType> {
        match self {
            Self::Dynamic(e) => {
                Evaluator::Dynamic(Box::new(e) as Box<dyn EvalDynamic<T::StateType>>)
            }
            Self::Static(e) => Evaluator::Static(e.into_rc_state_type()),
        }
    }
}

pub enum EvalResult<T> {
    Dynamic(T),
    Static(Rc<T>),
}

pub trait Eval<T> {
    fn eval_alpha(&self, alpha: f32) -> EvalResult<T>;
}

impl<T> Eval<T> for Evaluator<T> {
    fn eval_alpha(&self, alpha: f32) -> EvalResult<T> {
        match self {
            Self::Dynamic(e) => EvalResult::Dynamic(e.eval_alpha(alpha)),
            Self::Static(e) => EvalResult::Static(e.clone()),
        }
    }
}

// Mark: Animations
// MARK: AnimSchedule

pub struct AnimSchedule<'r, 't, T> {
    pub(crate) rabject: &'r mut Rabject<'t, T>,
    pub(crate) evaluator: Evaluator<T>,
    pub(crate) params: AnimParams,
}

impl<'r, 't, T: 'static> AnimSchedule<'r, 't, T> {
    pub fn new(rabject: &'r mut Rabject<'t, T>, evaluator: Evaluator<T>) -> Self {
        Self {
            rabject,
            evaluator,
            params: AnimParams::default(),
        }
    }
    pub fn with_duration(mut self, duration_secs: f32) -> Self {
        self.params.duration_secs = duration_secs;
        self
    }
    pub fn with_rate_func(mut self, rate_func: fn(f32) -> f32) -> Self {
        self.params.rate_func = rate_func;
        self
    }
}

impl<T: EntityTimelineStaticState + Clone + 'static> AnimSchedule<'_, '_, T> {
    pub fn apply(self) -> Self {
        if let Evaluator::Dynamic(evaluator) = &self.evaluator {
            self.rabject.data = evaluator.as_ref().eval_alpha(1.0);
        }
        self.rabject.timeline.update(self.rabject);
        self
    }
}

// MARK: AnimParams

/// The param of an animation
#[derive(Debug, Clone)]
pub struct AnimParams {
    /// Default: 1.0
    pub duration_secs: f32,
    /// Default: linear
    pub rate_func: fn(f32) -> f32,
}

impl Default for AnimParams {
    fn default() -> Self {
        Self {
            duration_secs: 1.0,
            rate_func: linear,
        }
    }
}
