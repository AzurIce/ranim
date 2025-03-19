//! Evaluation and animation
pub mod creation;
pub mod fading;
pub mod transform;

use crate::{
    items::{Rabject, group::Group},
    timeline::EntityTimelineStaticState,
    utils::rate_functions::linear,
};

#[allow(unused)]
use log::trace;
use std::{fmt::Debug, iter::Once, rc::Rc};

// MARK: Eval

pub trait EvalDynamic<T> {
    fn eval_alpha(&self, alpha: f32) -> T;
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
    Dynamic(Box<dyn EvalDynamic<T>>),
    Static(Rc<T>),
}

impl<T> From<Box<dyn EvalDynamic<T>>> for Evaluator<T> {
    fn from(value: Box<dyn EvalDynamic<T>>) -> Self {
        Self::Dynamic(value)
    }
}

impl<T> Evaluator<T> {
    pub fn new_dynamic<F: EvalDynamic<T> + 'static>(func: F) -> Self {
        Self::Dynamic(Box::new(func))
    }
}

#[derive(Debug)]
pub enum EvalResult<T> {
    Dynamic(T),
    Static(Rc<T>),
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

pub struct EvalAdapter<T> {
    pub inner: Box<dyn Eval<T>>,
}

impl<T: EntityTimelineStaticState + Clone + 'static> Eval<T::StateType> for EvalAdapter<T> {
    fn eval_alpha(&self, alpha: f32) -> EvalResult<T::StateType> {
        match self.inner.eval_alpha(alpha) {
            EvalResult::Dynamic(res) => EvalResult::Dynamic(res.into_state_type()),
            EvalResult::Static(res) => EvalResult::Static(res.into_rc_state_type()),
        }
    }
}

// MARK: Animation

pub struct ChainedAnimation<T> {
    anims: Vec<Animation<T>>,
    end_secs: Vec<f32>,
}

impl<T> ChainedAnimation<T> {
    /// len >= 1
    pub fn new(anims: Vec<Animation<T>>) -> Self {
        let mut end_secs = Vec::with_capacity(anims.len());
        let mut sum = 0.0;
        anims
            .iter()
            .map(|anim| anim.span_len())
            .for_each(|duration| {
                sum += duration;
                end_secs.push(sum);
            });

        Self { anims, end_secs }
    }
}

impl<T> Eval<T> for ChainedAnimation<T> {
    fn eval_alpha(&self, alpha: f32) -> EvalResult<T> {
        let inner_global_sec = self.end_secs.last().unwrap() * alpha;

        let (anim, end_sec) = self
            .anims
            .iter()
            .zip(self.end_secs.iter())
            .find(|&(_, end_sec)| *end_sec >= inner_global_sec)
            .unwrap();
        anim.eval_sec(inner_global_sec - end_sec + anim.span_len())
    }
}

impl<T: 'static> From<ChainedAnimation<T>> for Animation<T> {
    fn from(value: ChainedAnimation<T>) -> Self {
        Self {
            duration_secs: value.end_secs.last().cloned().unwrap(),
            evaluator: Box::new(value),
            padding: (0.0, 0.0),
            rate_func: linear,
        }
    }
}

pub struct Animation<T> {
    evaluator: Box<dyn Eval<T>>,
    pub(crate) rate_func: fn(f32) -> f32,
    pub(crate) duration_secs: f32,
    pub(crate) padding: (f32, f32),
}

impl<T> Debug for Animation<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Animation {{ duration_secs: {}, padding: {:?}, rate_func: {:?} }}",
            self.duration_secs, self.padding, self.rate_func
        )
    }
}

impl<T: EntityTimelineStaticState + Clone + 'static> Animation<T> {
    pub fn into_state_type(self) -> Animation<T::StateType> {
        Animation {
            evaluator: Box::new(EvalAdapter {
                inner: self.evaluator,
            }),
            rate_func: self.rate_func,
            duration_secs: self.duration_secs,
            padding: self.padding,
        }
    }
}

impl<T: 'static> Animation<T> {
    pub fn from_evaluator(evaluator: Evaluator<T>) -> Self {
        Self {
            evaluator: Box::new(evaluator),
            rate_func: linear,
            duration_secs: 1.0,
            padding: (0.0, 0.0),
        }
    }
}

impl<T> Animation<T> {
    pub fn span_len(&self) -> f32 {
        self.duration_secs + self.padding.0 + self.padding.1
    }
    pub fn with_rate_func(mut self, rate_func: fn(f32) -> f32) -> Self {
        self.rate_func = rate_func;
        self
    }
    pub fn with_duration(mut self, secs: f32) -> Self {
        self.duration_secs = secs;
        self
    }
    pub fn padding(mut self, start_sec: f32, end_sec: f32) -> Self {
        self.padding = (start_sec, end_sec);
        self
    }
    pub fn padding_start(mut self, sec: f32) -> Self {
        self.padding.0 = sec;
        self
    }
    pub fn padding_end(mut self, sec: f32) -> Self {
        self.padding.1 = sec;
        self
    }
    pub fn eval_alpha(&self, alpha: f32) -> EvalResult<T> {
        self.eval_sec(alpha * self.span_len())
    }
    pub fn eval_sec(&self, local_sec: f32) -> EvalResult<T> {
        self.evaluator.eval_alpha((self.rate_func)(
            ((local_sec - self.padding.0) / self.duration_secs).clamp(0.0, 1.0),
        ))
    }
}

// MARK: AnimSchedule

pub struct AnimSchedule<'r, 't, T> {
    pub(crate) rabject: &'r mut Rabject<'t, T>,
    pub(crate) anim: Animation<T>,
}

impl<T> Debug for AnimSchedule<'_, '_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AnimSchedule {{ rabject: {:?}, anim: {:?} }}",
            self.rabject.id, self.anim
        )
    }
}

impl<'r, 't, T: 'static> AnimSchedule<'r, 't, T> {
    pub fn new(rabject: &'r mut Rabject<'t, T>, anim: Animation<T>) -> Self {
        Self { rabject, anim }
    }
}

impl<T> AnimSchedule<'_, '_, T> {
    pub fn with_padding(mut self, start_sec: f32, end_sec: f32) -> Self {
        self.anim.padding = (start_sec, end_sec);
        self
    }
    pub fn with_rate_func(mut self, rate_func: fn(f32) -> f32) -> Self {
        self.anim.rate_func = rate_func;
        self
    }
    pub fn with_duration(mut self, secs: f32) -> Self {
        self.anim.duration_secs = secs;
        self
    }
}

impl<T: 'static> IntoIterator for AnimSchedule<'_, '_, T> {
    type IntoIter = Once<Self>;
    type Item = Self;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}

// MARK: Group

impl<T: 'static> Group<AnimSchedule<'_, '_, T>> {
    /// Sets the rate function for each animation in the group
    pub fn with_rate_func(mut self, rate_func: fn(f32) -> f32) -> Self {
        self.iter_mut().for_each(|schedule| {
            schedule.anim.rate_func = rate_func;
        });
        self
    }
    /// Sets the duration for each animation in the group
    pub fn with_duration(mut self, secs: f32) -> Self {
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
    pub fn with_total_duration(mut self, secs: f32) -> Self {
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

impl<T: Clone + 'static> AnimSchedule<'_, '_, T> {
    #[deprecated]
    pub fn chain(self, anim_builder: impl FnOnce(T) -> Animation<T>) -> Self {
        let AnimSchedule { rabject, anim } = self;
        let data = anim.eval_alpha(1.0).into_owned();
        let next_anim = (anim_builder)(data);
        let chained_anim = ChainedAnimation::new(vec![anim, next_anim]);
        Self {
            rabject,
            anim: chained_anim.into(),
        }
    }
}

impl<T: EntityTimelineStaticState + Clone + 'static> AnimSchedule<'_, '_, T> {
    pub fn apply(self) -> Self {
        if let EvalResult::Dynamic(res) = self.anim.eval_alpha(1.0) {
            self.rabject.data = res;
        }
        self
    }
}

impl<T: EntityTimelineStaticState + Clone + 'static> Group<AnimSchedule<'_, '_, T>> {
    pub fn apply(mut self) -> Self {
        self.iter_mut().for_each(|schedule| {
            if let EvalResult::Dynamic(res) = schedule.anim.eval_alpha(1.0) {
                schedule.rabject.data = res;
            }
            schedule.rabject.timeline.update(schedule.rabject);
        });
        self
    }
}
