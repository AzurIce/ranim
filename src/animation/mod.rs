pub mod creation;
pub mod fading;
pub mod transform;

use crate::{
    eval::Evaluator, items::{Entity, Rabject}, prelude::Timeline, timeline::EntityTimelineStaticState, utils::rate_functions::linear
};

#[allow(unused)]
use log::trace;
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
            evaluator: evaluator.into(),
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
            self.rabject.data = evaluator.eval_alpha(1.0);
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

impl AnimParams {
    pub fn with_duration(mut self, duration_secs: f32) -> Self {
        self.duration_secs = duration_secs;
        self
    }
    pub fn with_rate_func(mut self, rate_func: fn(f32) -> f32) -> Self {
        self.rate_func = rate_func;
        self
    }
}
