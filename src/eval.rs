use std::rc::Rc;

use crate::items::{Entity, P};


pub trait EvalDynamic<T> {
    fn eval_alpha(&self, alpha: f32) -> T;
}

pub enum Evaluator<T> {
    Dynamic(Box<dyn EvalDynamic<T>>),
    Static(Rc<T>),
}

impl<T> Evaluator<T> {
    pub fn new_dynamic<F: EvalDynamic<T> + 'static>(func: F) -> Self {
        Self::Dynamic(Box::new(func))
    }
}

pub enum EvalResult<T> {
    Dynamic(T),
    Static(Rc<T>),
}

pub trait Eval<T> {
    fn eval_alpha(&self, alpha: f32) -> EvalResult<T>;
}

impl<T: Entity + 'static> Eval<Box<dyn P>> for Evaluator<T> {
    fn eval_alpha(&self, alpha: f32) -> EvalResult<Box<dyn P>> {
        match self {
            Self::Dynamic(e) => EvalResult::Dynamic(Box::new(e.eval_alpha(alpha))),
            Self::Static(e) => EvalResult::Static(Rc::new(Box::new(e.clone())))
        }
    }
}

impl<T> Eval<T> for Evaluator<T> {
    fn eval_alpha(&self, alpha: f32) -> EvalResult<T> {
        match self {
            Self::Dynamic(e) => EvalResult::Dynamic(e.eval_alpha(alpha)),
            Self::Static(e) => EvalResult::Static(e.clone())
        }
    }
}

