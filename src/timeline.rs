use derive_more::{Deref, DerefMut};
use itertools::Itertools;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    animation::{AnimationSpan, EvalResult, Evaluator},
    items::{TimelineId, VisualItem, camera_frame::CameraFrame},
};
use std::fmt::Debug;
use std::{any::Any, sync::Arc};

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

#[derive(Deref, DerefMut)]
pub struct Timeline {
    id: usize,
    #[deref]
    #[deref_mut]
    inner: InnerTimeline,
}

impl Timeline {
    pub fn id(&self) -> usize {
        self.id
    }
}

impl TimelineFunc for Timeline {
    fn cur_sec(&self) -> f64 {
        self.inner.as_timeline().cur_sec()
    }
    fn elapsed_secs(&self) -> f64 {
        self.inner.as_timeline().elapsed_secs()
    }
    fn forward(&mut self, duration_secs: f64) {
        self.inner.as_timeline_mut().forward(duration_secs);
    }
    fn forward_to(&mut self, target_sec: f64) {
        self.inner.as_timeline_mut().forward_to(target_sec);
    }
    fn get_animation_infos(&self) -> Vec<AnimationInfo> {
        self.inner.as_timeline().get_animation_infos()
    }
    fn hide(&mut self) {
        self.inner.as_timeline_mut().hide();
    }
    fn seal(&mut self) {
        self.inner.as_timeline_mut().seal();
    }
    fn show(&mut self) {
        self.inner.as_timeline_mut().show();
    }
    fn show_secs(&self) -> &Vec<f64> {
        self.inner.as_timeline().show_secs()
    }
    fn type_name(&self) -> &str {
        self.inner.as_timeline().type_name()
    }
}

/// Timeline is a type erased [`ItemTimeline<T>`]
///
/// Currently There are two types of Timeline:
/// - [`InnerTimeline::VisualItem`]: Can be created from [`VisualItem`], has a boxed [`AnyVisualItemTimelineTrait`] in it.
/// - [`InnerTimeline::CameraFrame`]: Can be created from [`CameraFrame`], has a boxed [`AnyTimelineTrait`] in it.
pub enum InnerTimeline {
    CameraFrame(Box<dyn AnyTimelineTrait>),
    VisualItem(Box<dyn AnyVisualItemTimelineTrait>),
}

impl From<ItemTimeline<CameraFrame>> for InnerTimeline {
    fn from(value: ItemTimeline<CameraFrame>) -> Self {
        InnerTimeline::CameraFrame(Box::new(value))
    }
}

impl<T: VisualItem + Clone + 'static> From<ItemTimeline<T>> for InnerTimeline {
    fn from(value: ItemTimeline<T>) -> Self {
        InnerTimeline::VisualItem(Box::new(value))
    }
}

impl InnerTimeline {
    pub fn as_timeline(&self) -> &dyn TimelineFunc {
        match self {
            InnerTimeline::CameraFrame(timeline) => timeline.as_timeline(),
            InnerTimeline::VisualItem(timeline) => timeline.as_timeline(),
        }
    }
    pub fn as_timeline_mut(&mut self) -> &mut dyn TimelineFunc {
        match self {
            InnerTimeline::CameraFrame(timeline) => timeline.as_timeline_mut(),
            InnerTimeline::VisualItem(timeline) => timeline.as_timeline_mut(),
        }
    }
    pub fn as_any(&self) -> &dyn Any {
        match self {
            InnerTimeline::CameraFrame(timeline) => timeline.as_ref() as &dyn Any,
            InnerTimeline::VisualItem(timeline) => timeline.as_ref() as &dyn Any,
        }
    }
    pub fn as_any_mut(&mut self) -> &mut dyn Any {
        match self {
            InnerTimeline::CameraFrame(timeline) => timeline.as_mut() as &mut dyn Any,
            InnerTimeline::VisualItem(timeline) => timeline.as_mut() as &mut dyn Any,
        }
    }
}

// MARK: RanimScene
/// The main struct that offers the ranim's API, and encodes animations
/// The rabjects insert to it will hold a reference to it, so it has interior mutability
#[derive(Default)]
pub struct RanimScene {
    // Timeline<CameraFrame> or Timeline<Item>
    timelines: Vec<Timeline>,
    time_marks: Vec<(f64, TimeMark)>,
}

impl RanimScene {
    pub fn seal(mut self) -> SealedRanimScene {
        let total_secs = self.timelines.max_total_secs();
        self.timelines.forward_to(total_secs);
        self.timelines.seal();
        SealedRanimScene {
            total_secs,
            timelines: self.timelines,
            time_marks: self.time_marks,
        }
    }
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init_timeline<T: Clone + 'static>(&mut self, state: T) -> &mut ItemTimeline<T>
    where
        ItemTimeline<T>: Into<InnerTimeline>,
    {
        let id = TimelineId::alloc();
        self._make_sure_timeline_initialized::<T>(id.id(), state);
        self.timeline_mut(id)
    }
    pub fn timelines(&self) -> &Vec<Timeline> {
        &self.timelines
    }
    pub fn timelines_mut(&mut self) -> &mut Vec<Timeline> {
        &mut self.timelines
    }
    pub fn timeline<'a, T: TimelineIndex<'a>>(&'a self, index: T) -> T::RefOutput {
        index.timeline(self)
    }
    pub fn timeline_mut<'a, T: TimelineIndex<'a>>(&'a mut self, index: T) -> T::MutOutput {
        index.timeline_mut(self)
    }

    fn _make_sure_timeline_initialized<T: Clone + 'static>(&mut self, id: usize, state: T)
    where
        ItemTimeline<T>: Into<InnerTimeline>,
    {
        if self.timeline(id).is_none() {
            let rabject_timeline = ItemTimeline::<T>::new(TimelineId::new(id), state);
            self.timelines.push(Timeline {
                id,
                inner: rabject_timeline.into(),
            });
        }
    }

    pub fn insert_time_mark(&mut self, sec: f64, time_mark: TimeMark) {
        self.time_marks.push((sec, time_mark));
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RabjectTimelineInfo {
    pub id: usize,
    pub type_name: String,
    pub animation_infos: Vec<AnimationInfo>,
}

impl Debug for RanimScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Timeline: {} timelines", self.timelines.len()))?;
        Ok(())
    }
}

pub struct SealedRanimScene {
    total_secs: f64,
    timelines: Vec<Timeline>,
    time_marks: Vec<(f64, TimeMark)>,
}

impl SealedRanimScene {
    pub fn total_secs(&self) -> f64 {
        self.total_secs
    }
    pub fn time_marks(&self) -> &Vec<(f64, TimeMark)> {
        &self.time_marks
    }
    pub fn eval_sec(&self, target_sec: f64) -> TimelineEvalResult {
        let mut items = Vec::with_capacity(self.timelines.len());

        let mut camera_frame = None::<(EvalResult<CameraFrame>, usize)>;

        let eval_timeline = |timeline: &Timeline| match &timeline.inner {
            InnerTimeline::CameraFrame(inner) => {
                let timeline = (inner.as_ref() as &dyn Any)
                    .downcast_ref::<ItemTimeline<CameraFrame>>()
                    .unwrap();
                if let Some(res) = timeline.eval_sec(target_sec) {
                    camera_frame = Some(res)
                }
            }
            InnerTimeline::VisualItem(inner) => {
                if let Some((res, idx)) = inner.eval_sec(target_sec) {
                    items.push((timeline.id, res, idx));
                }
            }
        };
        self.timelines.iter().for_each(eval_timeline);
        // println!("alpha: {}, items: {}", alpha, items.len());
        // println!("alpha: {}, items: {}", alpha, items.len());

        TimelineEvalResult {
            camera_frame: camera_frame.unwrap(),
            visual_items: items,
        }
    }

    pub fn eval_alpha(&self, alpha: f64) -> TimelineEvalResult {
        self.eval_sec(alpha * self.total_secs)
    }

    pub fn get_timeline_infos(&self) -> Vec<RabjectTimelineInfo> {
        // const MAX_TIMELINE_CNT: usize = 100;
        self.timelines
            .iter()
            // .take(MAX_TIMELINE_CNT)
            .map(|timeline| RabjectTimelineInfo {
                id: timeline.id,
                type_name: timeline.as_timeline().type_name().to_string(),
                animation_infos: timeline.as_timeline().get_animation_infos(),
            })
            .collect()
    }
}

// MARK: TimelineIndex
pub trait TimelineIndex<'a> {
    type RefOutput;
    type MutOutput;
    fn timeline(self, timeline: &'a RanimScene) -> Self::RefOutput;
    fn timeline_mut(self, timeline: &'a mut RanimScene) -> Self::MutOutput;
}

impl<'a> TimelineIndex<'a> for usize {
    type RefOutput = Option<&'a Timeline>;
    type MutOutput = Option<&'a mut Timeline>;
    fn timeline(self, timeline: &'a RanimScene) -> Self::RefOutput {
        timeline
            .timelines()
            .iter()
            .find(|timeline| self == timeline.id)
    }
    fn timeline_mut(self, timeline: &'a mut RanimScene) -> Self::MutOutput {
        timeline
            .timelines_mut()
            .iter_mut()
            .find(|timeline| self == timeline.id)
    }
}

impl<'a, T: 'static> TimelineIndex<'a> for TimelineId<T> {
    type RefOutput = &'a ItemTimeline<T>;
    type MutOutput = &'a mut ItemTimeline<T>;
    fn timeline(self, timeline: &'a RanimScene) -> Self::RefOutput {
        timeline
            .timelines()
            .iter()
            .find(|timeline| self.id() == timeline.id)
            .map(|timeline| {
                timeline
                    .as_any()
                    .downcast_ref::<ItemTimeline<T>>()
                    .unwrap()
            })
            .unwrap()
    }
    fn timeline_mut(self, timeline: &'a mut RanimScene) -> Self::MutOutput {
        timeline
            .timelines_mut()
            .iter_mut()
            .find(|timeline| self.id() == timeline.id)
            .map(|timeline| {
                timeline
                    .as_any_mut()
                    .downcast_mut::<ItemTimeline<T>>()
                    .unwrap()
            })
            .unwrap()
    }
}

impl<'a, T: 'static, const N: usize> TimelineIndex<'a> for &[TimelineId<T>; N] {
    type RefOutput = [&'a ItemTimeline<T>; N];
    type MutOutput = [&'a mut ItemTimeline<T>; N];
    fn timeline(self, timeline: &'a RanimScene) -> Self::RefOutput {
        // TODO: the order is not stable
        let mut timelines = timeline
            .timelines()
            .iter()
            .filter(|timeline| self.iter().any(|rabject| rabject.id() == timeline.id))
            .collect_array::<N>()
            .unwrap();
        timelines
            .sort_by_key(|timeline| self.iter().position(|id| id.id() == timeline.id).unwrap());
        timelines.map(|timeline| {
            timeline
                .as_any()
                .downcast_ref::<ItemTimeline<T>>()
                .unwrap()
        })
    }
    fn timeline_mut(self, timeline: &'a mut RanimScene) -> Self::MutOutput {
        // TODO: the order is not stable
        let mut timelines = timeline
            .timelines_mut()
            .iter_mut()
            .filter(|timeline| self.iter().any(|rabject| rabject.id() == timeline.id))
            .collect_array::<N>()
            .unwrap();
        timelines
            .sort_by_key(|timeline| self.iter().position(|id| id.id() == timeline.id).unwrap());
        timelines.map(|timeline| {
            timeline
                .as_any_mut()
                .downcast_mut::<ItemTimeline<T>>()
                .unwrap()
        })
    }
}

// MARK: TimelinesFunc
pub trait TimelinesFunc {
    fn seal(&mut self);
    fn max_total_secs(&self) -> f64;
    fn sync(&mut self);
    fn forward(&mut self, secs: f64);
    fn forward_to(&mut self, target_sec: f64);
}

impl<I: ?Sized, T: TimelineFunc> TimelinesFunc for I
where
    for<'a> &'a mut I: IntoIterator<Item = &'a mut T>,
    for<'a> &'a I: IntoIterator<Item = &'a T>,
{
    fn seal(&mut self) {
        self.into_iter().for_each(|timeline: &mut T| {
            timeline.seal();
        });
    }
    fn max_total_secs(&self) -> f64 {
        self.into_iter()
            .map(|timeline: &T| timeline.cur_sec())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }
    fn sync(&mut self) {
        let max_elapsed_secs = self.max_total_secs();
        self.into_iter().for_each(|timeline: &mut T| {
            timeline.forward_to(max_elapsed_secs);
        });
    }
    fn forward(&mut self, secs: f64) {
        self.into_iter()
            .for_each(|timeline: &mut T| timeline.forward(secs));
    }
    fn forward_to(&mut self, target_sec: f64) {
        self.into_iter().for_each(|timeline: &mut T| {
            timeline.forward_to(target_sec);
        });
    }
}

// MARK: TimelineTrait
pub trait AnyTimelineTrait: TimelineFunc + Any {
    fn as_timeline(&self) -> &dyn TimelineFunc;
    fn as_timeline_mut(&mut self) -> &mut dyn TimelineFunc;
}
impl<T: TimelineFunc + Any> AnyTimelineTrait for T {
    fn as_timeline(&self) -> &dyn TimelineFunc {
        self
    }
    fn as_timeline_mut(&mut self) -> &mut dyn TimelineFunc {
        self
    }
}

pub trait AnyVisualItemTimelineTrait: VisualItemTimelineTrait + Any {
    fn as_timeline(&self) -> &dyn TimelineFunc;
    fn as_timeline_mut(&mut self) -> &mut dyn TimelineFunc;
}
impl<T: VisualItemTimelineTrait + Any> AnyVisualItemTimelineTrait for T {
    fn as_timeline(&self) -> &dyn TimelineFunc {
        self
    }
    fn as_timeline_mut(&mut self) -> &mut dyn TimelineFunc {
        self
    }
}

pub trait VisualItemTimelineTrait: TimelineFunc {
    fn eval_sec(&self, target_sec: f64) -> Option<(EvalResult<Box<dyn VisualItem>>, usize)>;
}

impl<T: Clone + VisualItem + 'static> VisualItemTimelineTrait for ItemTimeline<T> {
    fn eval_sec(&self, target_sec: f64) -> Option<(EvalResult<Box<dyn VisualItem>>, usize)> {
        let (item, idx) = self.eval_sec(target_sec)?;
        let item = item.map(|item| Box::new(item) as Box<dyn VisualItem>);
        Some((item, idx))
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AnimationInfo {
    pub anim_name: String,
    pub start_sec: f64,
    pub end_sec: f64,
}

// MARK: TimelineFunc
pub trait TimelineFunc {
    fn seal(&mut self);
    fn cur_sec(&self) -> f64;
    fn elapsed_secs(&self) -> f64;
    fn forward(&mut self, duration_secs: f64);
    fn forward_to(&mut self, target_sec: f64) {
        let duration = target_sec - self.cur_sec();
        if duration > 0.0 {
            self.forward(duration);
        }
    }
    // fn append_blank(&mut self, duration_secs: f64);
    // fn append_freeze(&mut self, duration_secs: f64);
    fn show(&mut self);
    fn hide(&mut self);
    fn get_animation_infos(&self) -> Vec<AnimationInfo>;
    fn type_name(&self) -> &str;
    fn show_secs(&self) -> &Vec<f64>;
}

// MARK: ItemTimeline
/// A timeline struct that encodes the animation of the type `T`
pub struct ItemTimeline<T> {
    id: TimelineId<T>,
    type_name: String,
    cur_sec: f64,
    /// The state used for static anim.
    ///
    /// It will be `Some` after the first [`ItemTimeline::update`] or [`ItemTimeline::update_with`]
    ///
    /// The [`ItemTimeline::show`] only works when it is `Some`.
    state: T,
    /// The start time of the planning static anim.
    /// When it is true, it means that it is showing.
    planning_static_start_sec: Option<f64>,

    animations: Vec<AnimationSpan<T>>,
    show_secs: Vec<f64>,
}

impl<T: Clone + 'static> TimelineFunc for ItemTimeline<T> {
    fn seal(&mut self) {
        // println!("seal");
        self._submit_planning_static_anim();
    }
    fn elapsed_secs(&self) -> f64 {
        self.show_secs.last().copied().unwrap_or(0.0)
    }
    fn cur_sec(&self) -> f64 {
        self.cur_sec
    }
    fn show_secs(&self) -> &Vec<f64> {
        &self.show_secs
    }
    /// The [`ItemTimeline::state`] should be `Some`
    fn show(&mut self) {
        // println!("show");
        if self.planning_static_start_sec.is_none() {
            self.planning_static_start_sec = Some(self.cur_sec)
        }
    }
    fn hide(&mut self) {
        // println!("hide");
        self._submit_planning_static_anim();
    }
    fn forward(&mut self, duration_secs: f64) {
        self.cur_sec += duration_secs;
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

impl<T: 'static> ItemTimeline<T> {
    /// Create a new timeline with the initial state
    ///
    /// The timeline is hidden by default, because we don't know when the first anim starts.
    /// And this allow us to use [`ItemTimeline::forward`] and [`ItemTimeline::forward_to`]
    /// to adjust the start time of the first anim.
    pub(crate) fn new(id: TimelineId<T>, state: T) -> Self {
        Self {
            id,
            state,
            type_name: std::any::type_name::<T>().to_string(),
            animations: vec![],
            planning_static_start_sec: None,
            cur_sec: 0.0,
            show_secs: vec![],
        }
    }
}

impl<T: Clone + 'static> ItemTimeline<T> {
    pub fn id(&self) -> TimelineId<T> {
        self.id
    }
    pub fn cur_sec(&self) -> f64 {
        self.cur_sec
    }
    pub fn state(&self) -> &T {
        &self.state
    }
    pub fn update_with(&mut self, update_func: impl FnOnce(&mut T)) {
        let showing = self._submit_planning_static_anim();
        update_func(&mut self.state);
        if showing {
            self.show();
        }
    }
    pub fn update(&mut self, state: T) {
        let showing = self._submit_planning_static_anim();
        self.state = state;
        if showing {
            self.show();
        }
    }
    fn push_anim(&mut self, anim: AnimationSpan<T>, start: f64, end: f64) {
        // println!("push_anim: {:?} ({}, {})", anim, start, end);
        self.animations.push(anim);
        self.show_secs.extend_from_slice(&[start, end]);
    }
    fn _submit_planning_static_anim(&mut self) -> bool {
        // println!("{:?}", self.planning_static_start_sec);
        if let Some(start) = self.planning_static_start_sec.take() {
            self.push_anim(
                AnimationSpan::from_evaluator(Evaluator::Static(Arc::new(self.state.clone()))),
                start,
                self.cur_sec,
            );
            return true;
        }
        false
    }
    pub fn play_with(&mut self, anim_func: impl FnOnce(T) -> AnimationSpan<T>) -> T {
        self.play(anim_func(self.state.clone()))
    }
    pub fn play(&mut self, anim: AnimationSpan<T>) -> T {
        self._submit_planning_static_anim();
        let res = anim.eval_alpha(1.0).into_owned();
        let duration = anim.span_len();
        let end = self.cur_sec + duration;
        self.push_anim(anim, self.cur_sec, end);
        self.cur_sec = end;
        self.update(res.clone());
        self.show();
        res
    }
    pub fn eval_alpha(&self, alpha: f64) -> Option<(EvalResult<T>, usize)> {
        let start = *self.show_secs.first().unwrap();
        let end = *self.show_secs.last().unwrap();
        self.eval_sec(alpha * (end - start) + start)
    }
    pub fn eval_sec(&self, target_sec: f64) -> Option<(EvalResult<T>, usize)> {
        if self.animations.is_empty() {
            return None;
        }

        if target_sec < *self.show_secs.first().unwrap()
            || target_sec > *self.show_secs.last().unwrap()
        {
            return None;
        }
        self.animations
            .iter()
            .zip(self.show_secs.chunks(2))
            .enumerate()
            .find_map(|(idx, (anim, show_secs))| {
                let start = show_secs.first().cloned().unwrap();
                let end = show_secs.get(1).cloned().unwrap_or(self.cur_sec());
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
