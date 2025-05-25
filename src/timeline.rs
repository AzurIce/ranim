use derive_more::{Deref, DerefMut};
use itertools::Itertools;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    animation::{AnimationSchedule, AnimationSpan, EvalResult, Evaluator},
    items::{TimelineId, VisualItem, camera_frame::CameraFrame},
};
use std::{any::Any, ops::Index, rc::Rc};
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

// // MARK: TimelineItem
// pub struct ItemMark;
// pub struct GroupMark;

// // MARK: TimelineAnim
// pub trait TimelineAnim<Mark> {
//     type Output;
//     fn schedule(self, timeline: &mut RanimScene) -> (Self::Output, f64);
// }

// // For AnimationSpan, we first insert it, then play and hide the anim
// impl<T: Clone + 'static> TimelineAnim<ItemMark> for AnimationSchedule<T>
// where
//     RabjectTimeline<T>: Into<InnerTimeline>,
// {
//     type Output = T;
//     fn schedule(self, timeline: &mut RanimScene) -> (Self::Output, f64) {
//         let cur_time = timeline.cur_sec();
//         let duration = self.inner.span_len();

//         let res = self.inner.eval_alpha(1.0).into_owned();
//         timeline._schedule(self.rabject_id, self.inner);
//         (res, cur_time + duration)
//     }
// }

// impl<E> TimelineAnim<GroupMark> for Vec<E>
// where
//     E: TimelineAnim<ItemMark>,
// {
//     type Output = Vec<E::Output>;
//     fn schedule(self, timeline: &mut RanimScene) -> (Self::Output, f64) {
//         let (results, end_times): (Vec<_>, Vec<_>) =
//             self.into_iter().map(|item| item.schedule(timeline)).unzip();
//         (
//             results,
//             end_times
//                 .into_iter()
//                 .max_by(|a, b| a.partial_cmp(b).unwrap())
//                 .unwrap(),
//         )
//     }
// }

// impl<T: TimelineAnim<ItemMark>, const N: usize> TimelineAnim<GroupMark> for [T; N] {
//     type Output = [T::Output; N];
//     fn schedule(self, timeline: &mut RanimScene) -> (Self::Output, f64) {
//         let (results, end_times): (Vec<_>, Vec<_>) =
//             self.into_iter().map(|item| item.schedule(timeline)).unzip();
//         (
//             results
//                 .try_into()
//                 .map_err(|_| anyhow::anyhow!("failed to convert vec to array"))
//                 .unwrap(),
//             end_times
//                 .into_iter()
//                 .max_by(|a, b| a.partial_cmp(b).unwrap())
//                 .unwrap(),
//         )
//     }
// }

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

impl TimelineTrait for Timeline {
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

/// Timeline is a type erased [`RabjectTimeline<T>`]
///
/// Currently There are two types of Timeline:
/// - [`Timeline::VisualItem`]: Can be created from [`VisualItem`], has a boxed [`AnyVisualItemTimelineTrait`] in it.
/// - [`Timeline::CameraFrame`]: Can be created from [`CameraFrame`], has a boxed [`AnyTimelineTrait`] in it.
pub enum InnerTimeline {
    CameraFrame(Box<dyn AnyTimelineTrait>),
    VisualItem(Box<dyn AnyVisualItemTimelineTrait>),
}

impl From<RabjectTimeline<CameraFrame>> for InnerTimeline {
    fn from(value: RabjectTimeline<CameraFrame>) -> Self {
        InnerTimeline::CameraFrame(Box::new(value))
    }
}

impl<T: VisualItem + Clone + 'static> From<RabjectTimeline<T>> for InnerTimeline {
    fn from(value: RabjectTimeline<T>) -> Self {
        InnerTimeline::VisualItem(Box::new(value))
    }
}

impl InnerTimeline {
    pub fn as_timeline(&self) -> &dyn TimelineTrait {
        match self {
            InnerTimeline::CameraFrame(timeline) => timeline.as_timeline(),
            InnerTimeline::VisualItem(timeline) => timeline.as_timeline(),
        }
    }
    pub fn as_timeline_mut(&mut self) -> &mut dyn TimelineTrait {
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

// MARK: Ranim
/// The main struct that offers the ranim's API, and encodes animations
/// The rabjects insert to it will hold a reference to it, so it has interior mutability
#[derive(Default)]
pub struct RanimScene {
    // Timeline<CameraFrame> or Timeline<Item>
    timelines: Vec<Timeline>,
    cur_secs: f64,
    time_marks: Vec<(f64, TimeMark)>,
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
        self.timelines
            .iter()
            .for_each(|timeline| match &timeline.inner {
                InnerTimeline::CameraFrame(inner) => {
                    let timeline = (inner.as_ref() as &dyn Any)
                        .downcast_ref::<RabjectTimeline<CameraFrame>>()
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
            });
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

pub trait TimelineIndex<'a> {
    type RefOutput;
    type MutOutput;
    fn timeline(&self, timeline: &'a RanimScene) -> Self::RefOutput;
    fn timeline_mut(&self, timeline: &'a mut RanimScene) -> Self::MutOutput;
}

impl<'a> TimelineIndex<'a> for usize {
    type RefOutput = Option<&'a Timeline>;
    type MutOutput = Option<&'a mut Timeline>;
    fn timeline(&self, timeline: &'a RanimScene) -> Self::RefOutput {
        timeline
            .timelines()
            .into_iter()
            .find(|timeline| *self == timeline.id)
    }
    fn timeline_mut(&self, timeline: &'a mut RanimScene) -> Self::MutOutput {
        timeline
            .timelines_mut()
            .into_iter()
            .find(|timeline| *self == timeline.id)
    }
}

impl<'a, T: 'static> TimelineIndex<'a> for TimelineId<T> {
    type RefOutput = &'a RabjectTimeline<T>;
    type MutOutput = &'a mut RabjectTimeline<T>;
    fn timeline(&self, timeline: &'a RanimScene) -> Self::RefOutput {
        timeline
            .timelines()
            .into_iter()
            .find(|timeline| self.id() == timeline.id)
            .map(|timeline| {
                timeline
                    .as_any()
                    .downcast_ref::<RabjectTimeline<T>>()
                    .unwrap()
            })
            .unwrap()
    }
    fn timeline_mut(&self, timeline: &'a mut RanimScene) -> Self::MutOutput {
        timeline
            .timelines_mut()
            .into_iter()
            .find(|timeline| self.id() == timeline.id)
            .map(|timeline| {
                timeline
                    .as_any_mut()
                    .downcast_mut::<RabjectTimeline<T>>()
                    .unwrap()
            })
            .unwrap()
    }
}

impl<'a, T: 'static, const N: usize> TimelineIndex<'a> for [&TimelineId<T>; N] {
    type RefOutput = [&'a RabjectTimeline<T>; N];
    type MutOutput = [&'a mut RabjectTimeline<T>; N];
    fn timeline(&self, timeline: &'a RanimScene) -> Self::RefOutput {
        // TODO: the order is not stable
        let mut timelines = timeline
            .timelines()
            .into_iter()
            .filter(|timeline| self.iter().any(|rabject| rabject.id() == timeline.id))
            .collect_array::<N>()
            .unwrap();
        timelines
            .sort_by_key(|timeline| self.iter().position(|id| id.id() == timeline.id).unwrap());
        timelines.map(|timeline| {
            timeline
                .as_any()
                .downcast_ref::<RabjectTimeline<T>>()
                .unwrap()
        })
    }
    fn timeline_mut(&self, timeline: &'a mut RanimScene) -> Self::MutOutput {
        // TODO: the order is not stable
        let mut timelines = timeline
            .timelines_mut()
            .into_iter()
            .filter(|timeline| self.iter().any(|rabject| rabject.id() == timeline.id))
            .collect_array::<N>()
            .unwrap();
        timelines
            .sort_by_key(|timeline| self.iter().position(|id| id.id() == timeline.id).unwrap());
        timelines.map(|timeline| {
            timeline
                .as_any_mut()
                .downcast_mut::<RabjectTimeline<T>>()
                .unwrap()
        })
    }
}

pub trait TimelinesFunc {
    fn seal(&mut self);
    fn max_total_secs(&self) -> f64;
    fn sync(&mut self);
    fn forward(&mut self, secs: f64);
    fn forward_to(&mut self, target_sec: f64);
}

impl<I: ?Sized, T: TimelineTrait> TimelinesFunc for I
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

    pub fn init_timeline<T: Clone + 'static>(&mut self, state: T) -> TimelineId<T>
    where
        RabjectTimeline<T>: Into<InnerTimeline>,
    {
        let rabject = TimelineId::new();
        self._make_sure_timeline_initialized::<T>(rabject.id(), state);
        rabject
    }
    pub fn timelines(&self) -> &Vec<Timeline> {
        &self.timelines
    }
    pub fn timelines_mut(&mut self) -> &mut Vec<Timeline> {
        &mut self.timelines
    }
    pub fn timeline<'a, T: TimelineIndex<'a>>(&'a self, index: &T) -> T::RefOutput {
        index.timeline(self)
    }
    pub fn timeline_mut<'a, T: TimelineIndex<'a>>(&'a mut self, index: &T) -> T::MutOutput {
        index.timeline_mut(self)
    }

    pub fn cur_sec(&self) -> f64 {
        self.cur_secs
    }

    fn _make_sure_timeline_initialized<T: Clone + 'static>(&mut self, id: usize, state: T)
    where
        RabjectTimeline<T>: Into<InnerTimeline>,
    {
        if !self.timeline(&id).is_some() {
            let mut rabject_timeline = RabjectTimeline::<T>::new(state);
            rabject_timeline.forward_to(self.cur_secs);
            self.timelines.push(Timeline {
                id,
                inner: rabject_timeline.into(),
            });
        }
    }

    // /// Show a rabject
    // ///
    // /// [`RanimTimeline::forward`] after this will encode timeline's static rabject state into the timeline
    // pub fn show<T: Clone + 'static>(&mut self, rabject: &RabjectId<T>)
    // where
    //     RabjectTimeline<T>: Into<InnerTimeline>,
    // {
    //     // self._make_sure_timeline_initialized(rabject.id());
    //     self._update(rabject.id(), rabject.data.clone());
    //     self._show(rabject.id());
    // }
    // fn _update<T: Clone + 'static>(&mut self, id: usize, state: T) {
    //     self.timeline_mut(&id)
    //         .unwrap()
    //         .as_any_mut()
    //         .downcast_mut::<RabjectTimeline<T>>()
    //         .unwrap()
    //         .update_state(state);
    // }
    // fn _show(&mut self, id: usize) {
    //     self.timeline_mut(&id).unwrap().as_timeline_mut().show();
    // }

    // /// Hide a rabject
    // ///
    // /// [`RanimTimeline::forward`] after this will encode blank into the timeline
    // pub fn hide<T: Clone + 'static>(&mut self, rabject: &RabjectId<T>)
    // where
    //     RabjectTimeline<T>: Into<InnerTimeline>,
    // {
    //     // self._make_sure_timeline_initialized(rabject.id());
    //     self._hide(rabject.id());
    // }
    // fn _hide(&mut self, id: usize) {
    //     self.timeline_mut(&id).unwrap().as_timeline_mut().hide();
    // }

    // /// Push an animation into the timeline
    // pub fn play<Mark, T: TimelineAnim<Mark>>(&mut self, anim: T) -> T::Output {
    //     let (res, end_sec) = self.schedule(anim);
    //     // trace!("play, end_sec: {}", end_sec);
    //     self.forward_to(end_sec);
    //     res
    // }

    // /// Push an animation into the timeline while not forwarding the timeline
    // ///
    // /// It will return the animation result and the end time of the animation
    // pub fn schedule<Mark, T: TimelineAnim<Mark>>(&mut self, anim: T) -> (T::Output, f64) {
    //     anim.schedule(self)
    // }

    // fn _schedule<T: Clone + 'static>(&mut self, id: usize, anim: AnimationSpan<T>) -> &Self
    // where
    //     RabjectTimeline<T>: Into<InnerTimeline>,
    // {
    //     // self._make_sure_timeline_initialized(id);
    //     // trace!("[RanimTimeline::_schedule] schedule {:?} on {:?}", anim, id);
    //     let timeline = self
    //         .timeline_mut(&id)
    //         .unwrap()
    //         .as_any_mut()
    //         .downcast_mut::<RabjectTimeline<T>>()
    //         .unwrap();

    //     timeline.play(anim);
    //     self
    // }

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
        f.write_fmt(format_args!(
            "Timeline {:?}: {} timelines\n",
            Duration::from_secs_f64(self.cur_sec()),
            self.timelines.len()
        ))?;
        Ok(())
    }
}

// MARK: TimelineTrait
pub trait AnyTimelineTrait: TimelineTrait + Any {
    fn as_timeline(&self) -> &dyn TimelineTrait;
    fn as_timeline_mut(&mut self) -> &mut dyn TimelineTrait;
}
impl<T: TimelineTrait + Any> AnyTimelineTrait for T {
    fn as_timeline(&self) -> &dyn TimelineTrait {
        self
    }
    fn as_timeline_mut(&mut self) -> &mut dyn TimelineTrait {
        self
    }
}

pub trait AnyVisualItemTimelineTrait: VisualItemTimelineTrait + Any {
    fn as_timeline(&self) -> &dyn TimelineTrait;
    fn as_timeline_mut(&mut self) -> &mut dyn TimelineTrait;
}
impl<T: VisualItemTimelineTrait + Any> AnyVisualItemTimelineTrait for T {
    fn as_timeline(&self) -> &dyn TimelineTrait {
        self
    }
    fn as_timeline_mut(&mut self) -> &mut dyn TimelineTrait {
        self
    }
}

pub trait VisualItemTimelineTrait: TimelineTrait {
    fn eval_sec(&self, target_sec: f64) -> Option<(EvalResult<Box<dyn VisualItem>>, usize)>;
}

impl<T: Clone + VisualItem + 'static> VisualItemTimelineTrait for RabjectTimeline<T> {
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

pub trait TimelineTrait {
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

// MARK: RabjectTimeline
/// A timeline struct that encodes the animation of the type `T`
pub struct RabjectTimeline<T> {
    type_name: String,
    cur_sec: f64,
    /// The state used for static anim.
    ///
    /// It will be `Some` after the first [`RabjectTimeline::update_state`]
    ///
    /// The [`RabjectTimeline::show`] only works when it is `Some`.
    state: T,
    /// The start time of the planning static anim.
    /// When it is true, it means that it is showing.
    planning_static_start_sec: Option<f64>,

    animations: Vec<AnimationSpan<T>>,
    show_secs: Vec<f64>,
}

impl<T: Clone + 'static> TimelineTrait for RabjectTimeline<T> {
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
    /// The [`RabjectTimeline::state`] should be `Some`
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

impl<T: 'static> RabjectTimeline<T> {
    /// Create a new timeline with the initial state
    ///
    /// The timeline is hidden by default, because we don't know when the first anim starts.
    /// And this allow us to use [`RabjectTimeline::forward`] and [`RabjectTimeline::forward_to`]
    /// to adjust the start time of the first anim.
    pub fn new(state: T) -> Self {
        Self {
            state,
            type_name: std::any::type_name::<T>().to_string(),
            animations: vec![],
            planning_static_start_sec: None,
            cur_sec: 0.0,
            show_secs: vec![],
        }
    }
}

impl<T: Clone + 'static> RabjectTimeline<T> {
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
                AnimationSpan::from_evaluator(Evaluator::Static(Rc::new(self.state.clone()))),
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
