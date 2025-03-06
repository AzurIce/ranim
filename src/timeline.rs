use crate::{
    // animation::{blank::Blank, AnimParams, AnimSchedule},
    animation::AnimSchedule,
    eval::{Eval, EvalResult, Evaluator},
    items::{Entity, Rabject, P},
    utils::{rate_functions::linear, Id}, // ItemData,
};
use std::{cell::RefCell, rc::Rc};
use std::{fmt::Debug, time::Duration};

pub use ranim_macros::timeline;

// MARK: Timeline

pub struct Timeline {
    rabject_timelines: RefCell<Vec<(Id, EvalTimeline<Box<dyn P>>, bool)>>,
    duration_secs: RefCell<f32>,
}

impl Timeline {
    pub fn eval_alpha(&self, alpha: f32) -> Vec<(Id, EvalResult<Box<dyn P>>, usize)> {
        let timelines = self.rabject_timelines.borrow();
        timelines
            .iter()
            .filter_map(|(id, timeline, _)| {
                timeline.eval_alpha(alpha).map(|(res, idx)| (*id, res, idx))
            })
            .collect::<Vec<_>>()
    }
}

impl Timeline {
    pub fn new() -> Self {
        Self {
            rabject_timelines: RefCell::new(Vec::new()),
            duration_secs: RefCell::new(0.0),
        }
    }
    pub fn duration_secs(&self) -> f32 {
        *self.duration_secs.borrow()
    }
    pub fn rabject_timelines_cnt(&self) -> usize {
        self.rabject_timelines.borrow().len()
    }
    pub fn insert<T: Entity + 'static>(&self, item: T) -> Rabject<T> {
        let mut timelines = self.rabject_timelines.borrow_mut();
        let rabject = Rabject::new(self, item);

        let timeline = if let Some((_, timeline, _)) =
            timelines.iter_mut().find(|(id, ..)| *id == rabject.id)
        {
            timeline
        } else {
            timelines.push((
                rabject.id,
                EvalTimeline::new(Box::new(rabject.data.clone())),
                true,
            ));
            timelines
                .last_mut()
                .map(|(_, timeline, _)| timeline)
                .unwrap()
        };
        if self.duration_secs() != 0.0 {
            timeline.append_blank(self.duration_secs());
        }
        rabject
    }
    pub fn update<T: Entity + 'static>(&self, rabject: &Rabject<T>) {
        let mut timelines = self.rabject_timelines.borrow_mut();
        let timeline = timelines
            .iter_mut()
            .find(|(id, ..)| *id == rabject.id)
            .unwrap();
        timeline
            .1
            .update_static_state(Some(Rc::new(Box::new(rabject.data.clone()))));
    }
    pub fn forward(&self, secs: f32) {
        self.rabject_timelines
            .borrow_mut()
            .iter_mut()
            .for_each(|(_id, timeline, is_showing)| {
                if *is_showing {
                    timeline.append_freeze(secs);
                } else {
                    timeline.append_blank(secs);
                }
            });
        *self.duration_secs.borrow_mut() += secs;
    }
    pub fn show<T: Entity>(&self, rabject: &Rabject<T>) {
        self.rabject_timelines
            .borrow_mut()
            .iter_mut()
            .find(|(id, _, _)| *id == rabject.id)
            .unwrap()
            .2 = true;
    }
    pub fn hide<T: Entity>(&self, rabject: &Rabject<T>) {
        self.rabject_timelines
            .borrow_mut()
            .iter_mut()
            .find(|(id, _, _)| *id == rabject.id)
            .unwrap()
            .2 = false;
    }
    /// Push an animation into the timeline
    ///
    /// Note that this won't apply the animation effect to rabject's data,
    /// to apply the animation effect use [`AnimSchedule::apply`]
    pub fn play<'t, T: Entity + 'static>(&'t self, anim_schedule: AnimSchedule<'_, 't, T>) {
        let mut timelines = self.rabject_timelines.borrow_mut();
        let AnimSchedule {
            rabject,
            evaluator,
            params,
        } = anim_schedule;
        // Fills the gap between the last animation and the current time
        let (id, timeline, idx) = timelines
            .iter_mut()
            .find(|(id, ..)| *id == rabject.id)
            .unwrap();

        // Fill the gap with its freeze
        let gapped_duration = self.duration_secs() - timeline.duration_secs();
        if gapped_duration > 0.0 {
            timeline.append_freeze(gapped_duration);
        }

        // Append the animation
        let duration = params.duration_secs;
        *self.duration_secs.borrow_mut() += duration;
        timeline.append(EvalTimelineElem {
            evaluator: Box::new(evaluator),
            rate_func: params.rate_func,
            duration_secs: params.duration_secs,
        });

        // Forword other timelines
        for (_id, timeline, is_showing) in timelines.iter_mut() {
            let secs = self.duration_secs() - timeline.duration_secs();
            if *is_showing {
                timeline.append_freeze(secs);
            } else {
                timeline.append_blank(secs);
            }
        }
    }
}

// /// Timeline of all rabjects
// ///
// /// Timeline has the interior mutability, and its [`Rabject`]s has the reference to it with the same lifetime.
// #[derive(Default)]
// pub struct Timeline {
//     /// Rabject's Id -> EntityTimeline
//     rabject_timelines: RefCell<HashMap<Id, EntityTimeline>>,
//     elapsed_secs: RefCell<f32>,
// }

impl Debug for Timeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Timeline {:?}:\n",
            Duration::from_secs_f32(self.duration_secs())
        ))?;
        for (id, timeline, _) in self.rabject_timelines.borrow().iter() {
            f.write_fmt(format_args!(
                "  EvalTimeline<Box<dyn P>>({:?}): {:?}\n",
                id,
                Duration::from_secs_f32(timeline.duration_secs())
            ))?;
        }
        Ok(())
    }
}

// MARK: EvalTimeline

pub struct EvalTimelineElem<T> {
    evaluator: Box<dyn Eval<T>>,
    rate_func: fn(f32) -> f32,
    duration_secs: f32,
}

impl<T> Eval<T> for EvalTimelineElem<T> {
    fn eval_alpha(&self, alpha: f32) -> EvalResult<T> {
        self.evaluator.eval_alpha((self.rate_func)(alpha))
    }
}

pub struct EvalTimeline<T> {
    forward_static_state: Option<Rc<T>>,
    elements: Vec<Option<EvalTimelineElem<T>>>,
    end_secs: Vec<f32>,
    is_showing: bool,
}

impl<T> EvalTimeline<T> {
    pub fn duration_secs(&self) -> f32 {
        self.end_secs.last().cloned().unwrap_or(0.0)
    }
}

impl<T: 'static> EvalTimeline<T> {
    pub fn new(initial_static_state: T) -> Self {
        Self {
            forward_static_state: Some(Rc::new(initial_static_state)),
            elements: Vec::new(),
            end_secs: Vec::new(),
            is_showing: true,
        }
    }
    pub fn update_static_state(&mut self, static_state: Option<Rc<T>>) {
        self.forward_static_state = static_state;
    }
    pub fn append(&mut self, elem: EvalTimelineElem<T>) {
        let end_state = match elem.eval_alpha(1.0) {
            EvalResult::Dynamic(res) => Rc::new(res),
            EvalResult::Static(res) => res,
        };
        self.update_static_state(Some(end_state));
        let end_sec = self.end_secs.last().copied().unwrap_or(0.0) + elem.duration_secs;
        self.elements.push(Some(elem));
        self.end_secs.push(end_sec);
    }
    pub fn append_blank(&mut self, duration_secs: f32) {
        // self.update_static_state(None);
        let end_sec = self.end_secs.last().copied().unwrap_or(0.0) + duration_secs;
        self.elements.push(None);
        self.end_secs.push(end_sec);
    }
    pub fn append_freeze(&mut self, duration_secs: f32) {
        let end_sec = self.end_secs.last().copied().unwrap_or(0.0) + duration_secs;
        self.elements.push(
            self.forward_static_state
                .as_ref()
                .map(|state| EvalTimelineElem {
                    evaluator: Box::new(Evaluator::Static(state.clone())),
                    rate_func: linear,
                    duration_secs,
                }),
        );
        self.end_secs.push(end_sec);
    }
}

impl<T> EvalTimeline<T> {
    pub fn eval_alpha(&self, alpha: f32) -> Option<(EvalResult<T>, usize)> {
        if self.elements.is_empty() {
            return None;
        }

        let alpha = alpha.clamp(0.0, 1.0);
        let target_sec = alpha * self.end_secs.last().unwrap();
        let (idx, (elem, end_sec)) = self
            .elements
            .iter()
            .zip(self.end_secs.iter())
            .enumerate()
            .find(|(_, (_, &end_sec))| end_sec >= target_sec)
            .unwrap();

        elem.as_ref().map(|elem| {
            let start_sec = end_sec - elem.duration_secs;
            let alpha = (target_sec - start_sec) / elem.duration_secs;
            (elem.eval_alpha(alpha), idx)
        })
    }
}
