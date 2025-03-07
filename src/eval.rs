//! Evaluating
use std::rc::Rc;

use crate::{
    items::{Entity, ItemEntity},
    timeline::{EntityTimelineStaticState, Item},
};

pub trait EvalDynamic<T> {
    fn eval_alpha(&self, alpha: f32) -> T;
}

pub enum Evaluator<T> {
    Dynamic(Box<dyn EvalDynamic<T>>),
    Static(Rc<T>),
}

impl<T: EntityTimelineStaticState + Clone + 'static> EvalDynamic<T::StateType>
    for Box<dyn EvalDynamic<T>>
{
    fn eval_alpha(&self, alpha: f32) -> T::StateType {
        self.as_ref().eval_alpha(alpha).into_state_type()
    }
}

// impl<T, F: EvalDynamic<T> + 'static> Into<Evaluator<T>> for F {
//     fn into(self) -> Evaluator<T> {
//         Evaluator::Dynamic(Box::new(func))
//     }
// }

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
