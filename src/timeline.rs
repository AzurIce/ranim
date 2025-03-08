use crate::{
    // animation::{blank::Blank, AnimParams, AnimSchedule},
    animation::{AnimParams, AnimSchedule},
    animation::{Eval, EvalResult, Evaluator},
    items::{camera_frame::CameraFrame, Entity, ItemEntity, Rabject},
    utils::rate_functions::linear, // ItemData,
};
use std::{any::Any, cell::RefCell, rc::Rc};
use std::{fmt::Debug, time::Duration};

use anyhow::anyhow;
pub use ranim_macros::timeline;

// MARK: Timeline

pub type Item = Box<dyn ItemEntity>;

pub struct Timelines<T>(Vec<EvalTimeline<T>>);

impl<T: 'static> Timelines<T> {
    pub fn insert(&mut self, data: T) -> usize {
        self.0.push(EvalTimeline::new(data));
        if let Some(duration_secs) = self.0.first().map(|timeline| timeline.duration_secs()) {
            self.0.last_mut().unwrap().append_blank(duration_secs);
        }
        self.0.len() - 1
    }
    pub fn update_static_state(&mut self, id: usize, state: Option<Rc<T>>) -> anyhow::Result<()> {
        let timeline = self
            .0
            .get_mut(id)
            .ok_or(anyhow!("Timeline with id {} not exist", id))?;
        timeline.update_static_state(state);
        Ok(())
    }
    pub fn forward(&mut self, secs: f32) {
        self.0.iter_mut().for_each(|timeline| {
            timeline.forward(secs);
        });
    }
    pub fn show(&mut self, id: usize) {
        self.0.get_mut(id).map(|timeline| {
            timeline.is_showing = true;
        });
    }
    pub fn hide(&mut self, id: usize) {
        self.0.get_mut(id).map(|timeline| {
            timeline.is_showing = false;
        });
    }
    pub fn play(
        &mut self,
        id: usize,
        evaluator: Box<dyn Eval<T>>,
        params: AnimParams,
    ) -> anyhow::Result<()> {
        let elapsed_time = self
            .0
            .first()
            .map(|timeline| timeline.duration_secs())
            .unwrap_or(0.0);

        let timeline = self
            .0
            .get_mut(id)
            .ok_or(anyhow!("Timeline with id {} not exist", id))?;
        // Fills the gap between the last animation and the current time

        // Fill the gap with its freeze
        let gapped_duration = elapsed_time - timeline.duration_secs();
        if gapped_duration > 0.0 {
            timeline.append_freeze(gapped_duration);
        }

        // Append the animation
        timeline.append_anim(Animation {
            evaluator,
            rate_func: params.rate_func,
            duration_secs: params.duration_secs,
        });

        // Forword other timelines
        self.0
            .iter_mut()
            .enumerate()
            .filter(|(_id, _)| *_id != id)
            .for_each(|(_, timeline)| {
                timeline.forward(params.duration_secs);
            });
        Ok(())
    }
}

impl<T> Timelines<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    /// entity_id, res, anim_id
    pub fn eval_alpha(&self, alpha: f32) -> Vec<(usize, EvalResult<T>, usize)> {
        self.0
            .iter()
            .enumerate()
            .filter_map(|(id, timeline)| {
                timeline.eval_alpha(alpha).map(|(res, idx)| (id, res, idx))
            })
            .collect::<Vec<_>>()
    }
    pub fn timelines_cnt(&self) -> usize {
        self.0.len()
    }
}

// MARK: EntityTimtlineState

pub trait EntityTimelineStaticState {
    type StateType;
    fn into_state_type(self) -> Self::StateType;
    fn into_rc_state_type(self: Rc<Self>) -> Rc<Self::StateType>;
    fn into_timeline(self) -> EvalTimeline<Self::StateType>
    where
        Self: Sized + 'static,
    {
        EvalTimeline::new(self.into_state_type())
    }
}

impl<T: Entity + 'static> EntityTimelineStaticState for T {
    type StateType = Item;
    fn into_rc_state_type(self: Rc<Self>) -> Rc<Self::StateType> {
        Rc::new(Box::new(self))
    }
    fn into_state_type(self) -> Self::StateType {
        Box::new(self)
    }
}

impl EntityTimelineStaticState for CameraFrame {
    type StateType = CameraFrame;
    fn into_rc_state_type(self: Rc<Self>) -> Rc<Self::StateType> {
        self.clone()
    }
    fn into_state_type(self) -> Self::StateType {
        self
    }
}

// MARK: Timeline

/// The main struct that offers the ranim's API, and encodes animations
/// The rabjects insert to it will hold a reference to it, so it has interior mutability
pub struct Timeline {
    // Timeline<CameraFrame> or Timeline<Item>
    timelines: RefCell<Vec<Box<dyn AnyEvalTimelineTrait>>>,
    duration_secs: RefCell<f32>,
}

impl<'a> Timeline {
    pub fn new() -> Self {
        Self {
            timelines: RefCell::new(Vec::new()),
            duration_secs: RefCell::new(0.0),
        }
    }
}

impl Timeline {
    pub fn duration_secs(&self) -> f32 {
        *self.duration_secs.borrow()
    }
    pub fn insert<T: EntityTimelineStaticState + Clone + 'static>(
        &self,
        static_state: T,
    ) -> Rabject<T> {
        let mut timelines = self.timelines.borrow_mut();

        let id = timelines.len();
        timelines.push(Box::new(static_state.clone().into_timeline()));
        Rabject {
            id,
            data: static_state,
            timeline: &self,
        }
    }
    pub fn update<T: EntityTimelineStaticState + Clone + 'static>(&self, rabject: &Rabject<T>) {
        let mut timelines = self.timelines.borrow_mut();
        let timeline = timelines
            .get_mut(rabject.id)
            .unwrap()
            .as_any_mut()
            .downcast_mut::<EvalTimeline<T::StateType>>()
            .unwrap();
        timeline.update_static_state(Some(Rc::new(rabject.data.clone().into_state_type())));
    }
    pub fn forward(&self, secs: f32) {
        self.timelines.borrow_mut().iter_mut().for_each(|timeline| {
            timeline.forward(secs);
        });
        *self.duration_secs.borrow_mut() += secs;
    }
    pub fn show<T>(&self, rabject: &Rabject<T>) {
        self.timelines
            .borrow_mut()
            .get_mut(rabject.id)
            .unwrap()
            .show();
    }
    pub fn hide<T>(&self, rabject: &Rabject<T>) {
        self.timelines
            .borrow_mut()
            .get_mut(rabject.id)
            .unwrap()
            .hide();
    }
    /// Push an animation into the timeline
    ///
    /// Note that this won't apply the animation effect to rabject's data,
    /// to apply the animation effect use [`AnimSchedule::apply`]
    pub fn play<'t, T: EntityTimelineStaticState + Clone + 'static>(
        &'t self,
        anim_schedule: AnimSchedule<'_, 't, T>,
    ) {
        *self.duration_secs.borrow_mut() += anim_schedule.params.duration_secs;

        let AnimSchedule {
            rabject,
            evaluator,
            params,
        } = anim_schedule;

        let mut timelines = self.timelines.borrow_mut();
        timelines
            .get_mut(rabject.id)
            .unwrap()
            .as_any_mut()
            .downcast_mut::<EvalTimeline<T::StateType>>()
            .unwrap()
            .append_anim(Animation {
                evaluator: Box::new(evaluator.to_state_type()),
                rate_func: params.rate_func,
                duration_secs: params.duration_secs,
            });
        timelines
            .iter_mut()
            .enumerate()
            .filter(|(id, _)| *id != rabject.id)
            .for_each(|(_, timeline)| {
                timeline.forward(params.duration_secs);
            });
    }
    pub fn eval_alpha(
        &self,
        alpha: f32,
    ) -> (
        (EvalResult<CameraFrame>, usize),
        Vec<(usize, EvalResult<Item>, usize)>,
    ) {
        let timelines = self.timelines.borrow_mut();

        let mut items = Vec::with_capacity(timelines.len());

        let mut camera_frame = None::<(EvalResult<CameraFrame>, usize)>;
        timelines.iter().enumerate().for_each(|(id, timeline)| {
            if let Some(timeline) = timeline
                .as_any()
                .downcast_ref::<EvalTimeline<CameraFrame>>()
            {
                camera_frame = timeline.eval_alpha(alpha)
            } else if let Some(timeline) = timeline.as_any().downcast_ref::<EvalTimeline<Item>>() {
                timeline
                    .eval_alpha(alpha)
                    .map(|(res, idx)| items.push((id, res, idx)));
            }
        });

        (camera_frame.unwrap(), items)
    }
}

impl Debug for Timeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Timeline {:?}: {} timelines\n",
            Duration::from_secs_f32(self.duration_secs()),
            self.timelines.borrow().len()
        ))?;
        Ok(())
    }
}

// MARK: Animation

pub struct Animation<T> {
    evaluator: Box<dyn Eval<T>>,
    rate_func: fn(f32) -> f32,
    duration_secs: f32,
}

impl<T> Eval<T> for Animation<T> {
    fn eval_alpha(&self, alpha: f32) -> EvalResult<T> {
        self.evaluator.eval_alpha((self.rate_func)(alpha))
    }
}

// MARK: EvalTimeline
/// A timeline struct that encodes the animation of the type `T`
pub struct EvalTimeline<T> {
    forward_static_state: Option<Rc<T>>,
    animations: Vec<Option<Animation<T>>>,
    end_secs: Vec<f32>,
    // Encoding state
    is_showing: bool,
}

pub trait AnyEvalTimelineTrait: EvalTimelineTrait + Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: EvalTimelineTrait + Any> AnyEvalTimelineTrait for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
pub trait EvalTimelineTrait {
    fn forward(&mut self, duration_secs: f32);
    fn append_blank(&mut self, duration_secs: f32);
    fn append_freeze(&mut self, duration_secs: f32);
    fn show(&mut self);
    fn hide(&mut self);
}

impl<T: 'static> EvalTimelineTrait for EvalTimeline<T> {
    fn show(&mut self) {
        self.is_showing = true;
    }
    fn hide(&mut self) {
        self.is_showing = false;
    }
    fn forward(&mut self, duration_secs: f32) {
        if self.is_showing {
            self.append_freeze(duration_secs);
        } else {
            self.append_blank(duration_secs);
        }
    }
    fn append_blank(&mut self, duration_secs: f32) {
        // self.update_static_state(None);
        let end_sec = self.end_secs.last().copied().unwrap_or(0.0) + duration_secs;
        self.animations.push(None);
        self.end_secs.push(end_sec);
    }
    fn append_freeze(&mut self, duration_secs: f32) {
        let end_sec = self.end_secs.last().copied().unwrap_or(0.0) + duration_secs;
        self.animations
            .push(self.forward_static_state.as_ref().map(|state| Animation {
                evaluator: Box::new(Evaluator::Static(state.clone())) as Box<dyn Eval<T>>,
                rate_func: linear,
                duration_secs,
            }));
        self.end_secs.push(end_sec);
    }
}

impl<T> EvalTimeline<T> {
    fn update_static_state(&mut self, static_state: Option<Rc<T>>) {
        self.forward_static_state = static_state;
    }
    fn append_anim(&mut self, anim: Animation<T>) {
        let end_state = match anim.eval_alpha(1.0) {
            EvalResult::Dynamic(res) => Rc::new(res),
            EvalResult::Static(res) => res,
        };
        self.update_static_state(Some(end_state));
        let end_sec = self.end_secs.last().copied().unwrap_or(0.0) + anim.duration_secs;
        self.animations.push(Some(anim));
        self.end_secs.push(end_sec);
    }
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
            animations: Vec::new(),
            end_secs: Vec::new(),
            is_showing: true,
        }
    }
}

impl<T> EvalTimeline<T> {
    pub fn eval_alpha(&self, alpha: f32) -> Option<(EvalResult<T>, usize)> {
        if self.animations.is_empty() {
            return None;
        }

        let alpha = alpha.clamp(0.0, 1.0);
        let target_sec = alpha * self.end_secs.last().unwrap();
        let (idx, (elem, end_sec)) = self
            .animations
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
