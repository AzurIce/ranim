use log::trace;

use crate::{
    animation::{AnimSchedule, Animation, EvalResult, Evaluator},
    items::{Entity, Rabject, camera_frame::CameraFrame, group::Group},
};
use std::{any::Any, cell::RefCell, rc::Rc};
use std::{fmt::Debug, time::Duration};

pub type Item = Box<dyn Entity>;

// MARK: EntityTimtlineState

pub trait EntityTimelineStaticState {
    type StateType;
    fn into_state_type(self) -> Self::StateType;
    fn into_rc_state_type(self: Rc<Self>) -> Rc<Self::StateType>;
    fn into_timeline(self) -> RabjectTimeline<Self::StateType>
    where
        Self: Sized + 'static,
    {
        RabjectTimeline::new(self.into_state_type())
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

// MARK: RanimTimeline

#[derive(Debug, Clone)]
pub enum TimeMark {
    Capture(String),
}

/// The main struct that offers the ranim's API, and encodes animations
/// The rabjects insert to it will hold a reference to it, so it has interior mutability
#[derive(Default)]
pub struct RanimTimeline {
    // Timeline<CameraFrame> or Timeline<Item>
    timelines: RefCell<Vec<Box<dyn AnyTimelineTrait>>>,
    max_elapsed_secs: RefCell<f32>,
    time_marks: RefCell<Vec<(f32, TimeMark)>>,
}

pub struct TimelineEvalResult {
    pub camera_frame: (EvalResult<CameraFrame>, usize),
    /// (id, EvalResult<Item>, animation idx in the corresponding timeline)
    pub items: Vec<(usize, EvalResult<Item>, usize)>,
}

impl RanimTimeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn duration_secs(&self) -> f32 {
        *self.max_elapsed_secs.borrow()
    }

    /// Push an animation into the timeline
    ///
    /// Note that this won't apply the animation effect to rabject's data,
    /// to apply the animation effect use [`AnimSchedule::apply`]
    pub fn play<'r, 't, T, I>(&'t self, anim_schedules: I) -> &'t Self
    where
        't: 'r,
        T: EntityTimelineStaticState + Clone + 'static,
        I: IntoIterator<Item = AnimSchedule<'r, 't, T>>,
    {
        anim_schedules.into_iter().for_each(|anim_schedule| {
            self._play(anim_schedule);
        });
        self
    }
    /// Insert an iterator of items into the timeline, and return a group of rabjects
    ///
    /// This is equivalent to
    /// ```rust
    /// items
    ///     .into_iter()
    ///     .map(|item| self.insert(item))
    ///     .collect::<Group<_>>()
    /// ```
    pub fn insert_group<'r, 't, T, I>(&'t self, items: I) -> Group<Rabject<'t, T>>
    where
        't: 'r,
        T: EntityTimelineStaticState + Clone + 'static,
        I: IntoIterator<Item = T>,
    {
        items.into_iter().map(|item| self.insert(item)).collect()
    }

    /// Insert an item into the timeline, and return a rabject
    pub fn insert<T: EntityTimelineStaticState + Clone + 'static>(&self, item: T) -> Rabject<T> {
        let mut timelines = self.timelines.borrow_mut();

        let id = timelines.len();
        let mut timeline = item.clone().into_timeline();
        let max_elapsed_secs = *self.max_elapsed_secs.borrow();
        if max_elapsed_secs != 0.0 {
            timeline.append_blank(*self.max_elapsed_secs.borrow());
        }
        timelines.push(Box::new(timeline));
        Rabject {
            id,
            data: item,
            timeline: self,
        }
    }

    /// Update the static state of a rabject's timeline
    pub fn update<T: EntityTimelineStaticState + Clone + 'static>(&self, rabject: &Rabject<T>) {
        let mut timelines = self.timelines.borrow_mut();
        let timeline = timelines
            .get_mut(rabject.id)
            .unwrap()
            .as_any_mut()
            .downcast_mut::<RabjectTimeline<T::StateType>>()
            .unwrap();
        timeline.update_static_state(Some(Rc::new(rabject.data.clone().into_state_type())));
    }
    /// Forward all rabjects' timeline by the given seconds
    pub fn forward(&self, secs: f32) -> &Self {
        self.timelines.borrow_mut().iter_mut().for_each(|timeline| {
            timeline.forward(secs);
        });
        *self.max_elapsed_secs.borrow_mut() += secs;
        self
    }

    /// Show a rabject
    ///
    /// [`RanimTimeline::forward`] after this will encode timeline's static rabject state into the timeline
    pub fn show<T>(&self, rabject: &Rabject<T>) {
        self.timelines
            .borrow_mut()
            .get_mut(rabject.id)
            .unwrap()
            .show();
    }
    /// Hide a rabject
    /// 
    /// [`RanimTimeline::forward`] after this will encode blank into the timeline
    pub fn hide<T>(&self, rabject: &Rabject<T>) {
        self.timelines
            .borrow_mut()
            .get_mut(rabject.id)
            .unwrap()
            .hide();
    }

    /// Sync all rabjects' timeline to the max elapsed seconds
    pub fn sync(&self) -> &Self {
        let mut timelines = self.timelines.borrow_mut();
        let max_elapsed_secs = self.max_elapsed_secs.borrow();
        timelines.iter_mut().for_each(|timeline| {
            // println!("{}, {}", timeline.elapsed_secs(), *max_elapsed_secs);
            if timeline.elapsed_secs() < *max_elapsed_secs {
                timeline.forward(*max_elapsed_secs - timeline.elapsed_secs());
            }
        });
        self
    }

    fn _play<'r, 't: 'r, T: EntityTimelineStaticState + Clone + 'static>(
        &'t self,
        anim_schedule: AnimSchedule<'r, 't, T>,
    ) -> &'t Self {
        trace!("play {:?}", anim_schedule);
        let mut timelines = self.timelines.borrow_mut();
        let AnimSchedule { rabject, anim } = anim_schedule;

        let timeline = timelines
            .get_mut(rabject.id)
            .unwrap()
            .as_any_mut()
            .downcast_mut::<RabjectTimeline<T::StateType>>()
            .unwrap();

        let mut max_duration = self.max_elapsed_secs.borrow_mut();
        timeline.append_anim(anim.into_state_type());
        *max_duration = max_duration.max(timeline.duration_secs());
        self
    }

    pub fn insert_time_mark(&self, sec: f32, time_mark: TimeMark) {
        self.time_marks.borrow_mut().push((sec, time_mark));
    }
    pub fn time_marks(&self) -> Vec<(f32, TimeMark)> {
        self.time_marks.borrow().clone()
    }
    pub fn eval_sec(&self, local_sec: f32) -> TimelineEvalResult {
        self.eval_alpha(local_sec / *self.max_elapsed_secs.borrow())
    }

    pub fn eval_alpha(&self, alpha: f32) -> TimelineEvalResult {
        let timelines = self.timelines.borrow_mut();

        let mut items = Vec::with_capacity(timelines.len());

        let mut camera_frame = None::<(EvalResult<CameraFrame>, usize)>;
        timelines.iter().enumerate().for_each(|(id, timeline)| {
            if let Some(timeline) = timeline
                .as_any()
                .downcast_ref::<RabjectTimeline<CameraFrame>>()
            {
                camera_frame = timeline.eval_alpha(alpha)
            } else if let Some(timeline) = timeline.as_any().downcast_ref::<RabjectTimeline<Item>>()
            {
                if let Some((res, idx)) = timeline.eval_alpha(alpha) {
                    items.push((id, res, idx));
                }
            }
        });

        TimelineEvalResult {
            camera_frame: camera_frame.unwrap(),
            items,
        }
    }
}

impl Debug for RanimTimeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Timeline {:?}: {} timelines\n",
            Duration::from_secs_f32(self.duration_secs()),
            self.timelines.borrow().len()
        ))?;
        Ok(())
    }
}

// MARK: RabjectTimeline

/// A timeline struct that encodes the animation of the type `T`
pub struct RabjectTimeline<T> {
    forward_static_state: Option<Rc<T>>,
    animations: Vec<Option<Animation<T>>>,
    end_secs: Vec<f32>,
    // Encoding states
    is_showing: bool,
}

pub trait AnyTimelineTrait: TimelineTrait + Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: TimelineTrait + Any> AnyTimelineTrait for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
pub trait TimelineTrait {
    fn elapsed_secs(&self) -> f32;
    fn forward(&mut self, duration_secs: f32);
    fn append_blank(&mut self, duration_secs: f32);
    fn append_freeze(&mut self, duration_secs: f32);
    fn show(&mut self);
    fn hide(&mut self);
}

impl<T: 'static> TimelineTrait for RabjectTimeline<T> {
    fn elapsed_secs(&self) -> f32 {
        self.end_secs.last().cloned().unwrap_or(0.0)
    }
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
        let end_sec = self.end_secs.last().copied().unwrap_or(0.0) + duration_secs;
        self.animations.push(None);
        self.end_secs.push(end_sec);
    }
    fn append_freeze(&mut self, duration_secs: f32) {
        let end_sec = self.end_secs.last().copied().unwrap_or(0.0) + duration_secs;
        self.animations.push(
            self.forward_static_state
                .as_ref()
                .map(|state| Animation::from_evaluator(Evaluator::Static(state.clone()))),
        );
        self.end_secs.push(end_sec);
    }
}

impl<T> RabjectTimeline<T> {
    fn update_static_state(&mut self, static_state: Option<Rc<T>>) {
        self.forward_static_state = static_state;
    }
    fn append_anim(&mut self, anim: Animation<T>) {
        let end_state = match anim.eval_alpha(1.0) {
            EvalResult::Dynamic(res) => Rc::new(res),
            EvalResult::Static(res) => res,
        };
        self.update_static_state(Some(end_state));
        let end_sec = self.end_secs.last().copied().unwrap_or(0.0) + anim.span_len();
        // println!("{}", end_sec);
        self.animations.push(Some(anim));
        self.end_secs.push(end_sec);
    }
}

impl<T> RabjectTimeline<T> {
    pub fn duration_secs(&self) -> f32 {
        self.end_secs.last().cloned().unwrap_or(0.0)
    }
}

impl<T: 'static> RabjectTimeline<T> {
    pub fn new(initial_static_state: T) -> Self {
        Self {
            forward_static_state: Some(Rc::new(initial_static_state)),
            animations: Vec::new(),
            end_secs: Vec::new(),
            is_showing: true,
        }
    }
}

impl<T> RabjectTimeline<T> {
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
            .find(|&(_, (_, &end_sec))| end_sec >= target_sec)
            .unwrap();

        elem.as_ref().map(|elem| {
            let start_sec = end_sec - elem.span_len();
            let alpha = (target_sec - start_sec) / elem.span_len();
            (elem.eval_alpha(alpha), idx)
        })
    }
}
