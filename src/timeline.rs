#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    animation::{AnimationSpan, EvalResult, Evaluator},
    items::{VisualItem, camera_frame::CameraFrame},
};
use std::{any::Any, cell::RefCell, rc::Rc};
use std::{fmt::Debug, time::Duration};

#[derive(Debug, Clone)]
pub enum TimeMark {
    Capture(String),
}

#[allow(clippy::type_complexity)]
pub struct TimelineEvalResult {
    pub camera_frame: (EvalResult<CameraFrame>, usize),
    /// (`id`, `EvalResult<Box<dyn RenderableItem>>`, `animation idx` in the corresponding timeline)
    pub visual_items: Vec<(usize, EvalResult<Box<dyn VisualItem>>, usize)>,
}

/// Timeline is a type erased [`RabjectTimeline<T>`]
///
/// Currently There are two types of Timeline:
/// - [`Timeline::VisualItem`]: Can be created from [`VisualItem`], has a boxed [`AnyVisualItemTimelineTrait`] in it.
/// - [`Timeline::CameraFrame`]: Can be created from [`CameraFrame`], has a boxed [`AnyTimelineTrait`] in it.
pub enum Timeline {
    CameraFrame(Box<dyn AnyTimelineTrait>),
    VisualItem(Box<dyn AnyVisualItemTimelineTrait>),
}

pub trait AnyTimelineTrait: TimelineTrait + Any {}
impl<T: TimelineTrait + Any> AnyTimelineTrait for T {}

pub trait AnyVisualItemTimelineTrait: VisualItemTimelineTrait + Any {}
impl<T: VisualItemTimelineTrait + Any> AnyVisualItemTimelineTrait for T {}

impl From<RabjectTimeline<CameraFrame>> for Timeline {
    fn from(value: RabjectTimeline<CameraFrame>) -> Self {
        Timeline::CameraFrame(Box::new(value))
    }
}

impl<T: VisualItem + Clone + 'static> From<RabjectTimeline<T>> for Timeline {
    fn from(value: RabjectTimeline<T>) -> Self {
        Timeline::VisualItem(Box::new(value))
    }
}

impl Timeline {
    pub fn as_timeline(&self) -> &dyn TimelineTrait {
        match self {
            Timeline::CameraFrame(timeline) => timeline.as_ref() as &dyn TimelineTrait,
            Timeline::VisualItem(timeline) => timeline.as_ref() as &dyn TimelineTrait,
        }
    }
    pub fn as_timeline_mut(&mut self) -> &mut dyn TimelineTrait {
        match self {
            Timeline::CameraFrame(timeline) => timeline.as_mut() as &mut dyn TimelineTrait,
            Timeline::VisualItem(timeline) => timeline.as_mut() as &mut dyn TimelineTrait,
        }
    }
}

// MARK: RanimTimeline
/// The main struct that offers the ranim's API, and encodes animations
/// The rabjects insert to it will hold a reference to it, so it has interior mutability
#[derive(Default)]
pub struct RanimTimeline {
    timelines: RefCell<Vec<Timeline>>,
    time_marks: RefCell<Vec<(f64, TimeMark)>>,
}

impl RanimTimeline {
    pub fn seal(self) -> SealedRanimTimeline {
        SealedRanimTimeline {
            total_secs: self.max_elapsed_secs(),
            timelines: self.timelines.take(),
            time_marks: self.time_marks.take(),
        }
    }
}

// MARK: SealedRanimTimeline
/// A sealed [`RanimTimeline`], can be construct with [`RanimTimeline::seal`].
///
/// Once the [`RanimTimeline`] is sealed, it cannot be modified anymore, and the total_secs is fixed.
pub struct SealedRanimTimeline {
    total_secs: f64,
    timelines: Vec<Timeline>,
    time_marks: Vec<(f64, TimeMark)>,
}

impl Debug for SealedRanimTimeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Timeline {:?}: {} timelines\n",
            Duration::from_secs_f64(self.total_secs),
            self.timelines.len()
        ))?;
        Ok(())
    }
}

impl SealedRanimTimeline {
    pub fn total_secs(&self) -> f64 {
        self.total_secs
    }
    pub fn time_marks(&self) -> &Vec<(f64, TimeMark)> {
        &self.time_marks
    }
    pub fn eval_sec(&self, sec: f64) -> TimelineEvalResult {
        let mut visual_items = Vec::with_capacity(self.timelines.len());

        let mut camera_frame = None::<(EvalResult<CameraFrame>, usize)>;
        self.timelines
            .iter().enumerate()
            .for_each(|(id, timeline)| match timeline {
                Timeline::CameraFrame(timeline) => {
                    let timeline = (timeline.as_ref() as &dyn Any)
                        .downcast_ref::<RabjectTimeline<CameraFrame>>()
                        .unwrap();
                    if let Some(res) = timeline.eval_sec(sec) {
                        camera_frame = Some(res)
                    }
                }
                Timeline::VisualItem(timeline) => {
                    if let Some((res, idx)) = timeline.eval_sec(sec) {
                        visual_items.push((id, res, idx));
                    }
                }
            });
        // println!("alpha: {}, items: {}", alpha, items.len());
        // println!("alpha: {}, items: {}", alpha, items.len());

        TimelineEvalResult {
            camera_frame: camera_frame.unwrap(),
            visual_items,
        }
    }

    pub fn eval_alpha(&self, alpha: f64) -> TimelineEvalResult {
        let sec = alpha * self.total_secs;
        self.eval_sec(sec)
    }

    pub fn get_timeline_infos(&self) -> Vec<RabjectTimelineInfo> {
        self.timelines
            .iter().enumerate()
            .map(|(id, timeline)| RabjectTimelineInfo {
                id,
                type_name: timeline.as_timeline().type_name().to_string(),
                animation_infos: timeline.as_timeline().get_animation_infos(),
            })
            .collect()
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RabjectTimelineInfo {
    pub id: usize,
    pub type_name: String,
    pub animation_infos: Vec<AnimationInfo>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AnimationInfo {
    pub anim_name: String,
    pub start_sec: f64,
    pub end_sec: f64,
}

pub struct SyncedTimelineEncoder<'t, 'a, T: Clone + 'static>
where
    RabjectTimeline<T>: Into<Timeline>,
{
    inner: & 'a mut TimelineEncoder<'t, T>,
}

impl<T: Clone + 'static> SyncedTimelineEncoder<T> {
    pub fn new_synced_sub_timeline(&)
}

impl<T: Clone + 'static> Drop for SyncedTimelineEncoder<'_, '_, T>
where
    RabjectTimeline<T>: Into<Timeline>,
{
    fn drop(&mut self) {
        let
    }
}

// MARK: TimelineEncoder
/// A timeline encoder that encodes the animation of the type `T`
///
/// It can be created with [`RanimTimeline::begin_timeline_encoder`] or [`TimelineEncoder::new_timeline_encoder`].
///
/// The created [`TimelineEncoder`] can be used to encode the animation of the type `T`.
///
/// When it is dropped, the encoded animations will be inserted into the [`RanimTimeline`].
pub struct TimelineEncoder<'t, T: Clone + 'static>
where
    RabjectTimeline<T>: Into<Timeline>,
{
    state: Option<T>,
    cur_sec: f64,
    /// This represents a static timeline span which is planning to be encoded
    /// For example, when we call `timeline.update(state)`, the state will not
    /// be encoded instantly, instead, it will wait until the next `update` or
    /// `play`, because the duration may be extended.
    static_span_start: Option<f64>,
    timeline: &'t RanimTimeline,
    inner: Option<RabjectTimeline<T>>,
}

impl<'a, T: Clone + 'static> TimelineEncoder<'a, T>
where
    RabjectTimeline<T>: Into<Timeline>,
{
    /// Get the current state data of the timeline
    pub fn state(&self) -> &Option<T> {
        &self.state
    }
    pub fn cur_sec(&mut self) -> f64 {
        self.cur_sec
    }
    /// Submits the planning static animation to the timeline
    fn submit_static_anim(&mut self) {
        if let Some(start) = self.static_span_start.take() {
            let inner = self.inner.as_mut().unwrap();
            inner.insert_anim(
                AnimationSpan::from_evaluator(Evaluator::Static(Rc::new(
                    self.state.as_ref().unwrap().clone(),
                ))),
                start,
                self.cur_sec,
            );
        }
    }
    pub fn update(&mut self, state: Option<T>) -> &mut Self {
        self.submit_static_anim();

        if state.is_some() {
            self.static_span_start = Some(self.cur_sec);
        }
        self.state = state;
        self
    }
    pub fn forward(&mut self, secs: f64) -> &mut Self {
        self.cur_sec += secs;
        self
    }
    pub fn forward_to(&mut self, sec: f64) -> &mut Self {
        if self.cur_sec < sec {
            self.forward(sec - self.cur_sec);
        }
        self
    }
    pub fn play(&mut self, anim: AnimationSpan<T>) -> T {
        self.submit_static_anim();

        let inner = self.inner.as_mut().unwrap();
        let dur = anim.span_len();
        let res = anim.eval_alpha(1.0).into_owned();
        self.state.replace(res.clone());
        inner.insert_anim(anim, self.cur_sec, self.cur_sec + dur);
        self.cur_sec += dur;
        res
    }
    pub fn new_timeline_encoder<E: Clone + 'static>(&self) -> TimelineEncoder<E>
    where
        RabjectTimeline<E>: Into<Timeline>,
    {
        TimelineEncoder {
            cur_sec: self.cur_sec,
            static_span_start: None,
            timeline: self.timeline,
            state: None,
            inner: Some(RabjectTimeline::new()),
        }
    }
}

impl<T: Clone + 'static> Drop for TimelineEncoder<'_, T>
where
    RabjectTimeline<T>: Into<Timeline>,
{
    fn drop(&mut self) {
        self.submit_static_anim();

        let inner = self.inner.take().unwrap();
        let cnt = self.timeline.timelines.borrow().len();
        self.timeline._insert_timeline(cnt, inner.into());
    }
}

impl RanimTimeline {
    pub fn begin_timeline_encoder<T: Clone + 'static>(&self) -> TimelineEncoder<T>
    where
        RabjectTimeline<T>: Into<Timeline>,
    {
        TimelineEncoder {
            cur_sec: 0.0,
            static_span_start: None,
            timeline: self,
            state: None,
            inner: Some(RabjectTimeline::new()),
        }
    }
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_elapsed_secs(&self) -> f64 {
        self.timelines
            .borrow()
            .iter()
            .map(|(_, timeline)| timeline.as_timeline().elapsed_secs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }

    fn _insert_timeline(&self, id: usize, timeline: Timeline) {
        let mut timelines = self.timelines.borrow_mut();
        timelines.push((id, timeline));
    }

    pub fn insert_time_mark(&self, sec: f64, time_mark: TimeMark) {
        self.time_marks.borrow_mut().push((sec, time_mark));
    }
    pub fn time_marks(&self) -> Vec<(f64, TimeMark)> {
        self.time_marks.borrow().clone()
    }
}

// MARK: TimelineTrait
pub trait TimelineTrait {
    fn elapsed_secs(&self) -> f64;
    fn get_animation_infos(&self) -> Vec<AnimationInfo>;
    fn type_name(&self) -> &str;
}
pub trait VisualItemTimelineTrait: TimelineTrait {
    fn eval_sec(&self, alpha: f64) -> Option<(EvalResult<Box<dyn VisualItem>>, usize)>;
}

impl<T: Clone + VisualItem + 'static> VisualItemTimelineTrait for RabjectTimeline<T> {
    fn eval_sec(&self, alpha: f64) -> Option<(EvalResult<Box<dyn VisualItem>>, usize)> {
        let (item, idx) = self.eval_sec(alpha)?;
        let item = item.map(|item| Box::new(item) as Box<dyn VisualItem>);
        Some((item, idx))
    }
}

// MARK: RabjectTimeline
/// A timeline struct that encodes the animation of the type `T`
pub struct RabjectTimeline<T> {
    type_name: String,
    animations: Vec<AnimationSpan<T>>,
    show_secs: Vec<f64>,
}

impl<T: Clone + 'static> TimelineTrait for RabjectTimeline<T> {
    fn elapsed_secs(&self) -> f64 {
        self.show_secs.last().unwrap_or(&0.0).clone()
    }
    fn get_animation_infos(&self) -> Vec<AnimationInfo> {
        // const MAX_INFO_CNT: usize = 100;
        self.animations
            .iter()
            .zip(self.show_secs.chunks(2))
            .map(|(anim, show_sec)| {
                let start = show_sec.first().unwrap();
                let end = show_sec.last().unwrap();
                AnimationInfo {
                    anim_name: anim.type_name.clone(),
                    start_sec: *start + anim.padding.0,
                    end_sec: *end - anim.padding.1,
                }
            })
            // .take(MAX_INFO_CNT)
            .collect()
    }
    fn type_name(&self) -> &str {
        &self.type_name
    }
}

impl<T: 'static> RabjectTimeline<T> {
    /// Create a new timeline with the initial state
    pub fn new() -> Self {
        Self {
            type_name: std::any::type_name::<T>().to_string(),
            animations: vec![],
            show_secs: vec![],
        }
    }
}

impl<T> RabjectTimeline<T> {
    /// Inserts an animation span into the timeline
    ///
    /// Should make sure that the show_secs is in even length
    fn insert_anim(&mut self, anim: AnimationSpan<T>, start_sec: f64, end_sec: f64) {
        assert!(self.show_secs.len() % 2 == 0);
        self.animations.push(anim);
        self.show_secs.extend_from_slice(&[start_sec, end_sec]);
    }

    pub fn eval_sec(&self, target_sec: f64) -> Option<(EvalResult<T>, usize)> {
        if self.animations.is_empty() {
            return None;
        }
        assert!(!self.animations.is_empty());
        assert!(!self.show_secs.is_empty());

        let target_sec = target_sec.min(*self.show_secs.last().unwrap());

        self.animations
            .iter()
            .zip(self.show_secs.chunks(2))
            .enumerate()
            .find_map(|(idx, (anim, show_secs))| {
                let start = show_secs.first().cloned().unwrap();
                let end = show_secs.get(1).cloned().unwrap_or(start);
                if start <= target_sec
                    && (target_sec < end || target_sec == end && idx == self.animations.len() - 1)
                {
                    Some((idx, anim, (start, end)))
                } else {
                    None
                }
            })
            .map(|(idx, anim, (start, end))| {
                let alpha = (target_sec - start) / (end - start);
                (anim.eval_alpha(alpha), idx)
            })
    }
}
