#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    animation::{AnimationSpan, EvalResult, Evaluator},
    items::{PinnedItem, VisualItem, camera_frame::CameraFrame, group::Group},
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

// MARK: TimelineItem
pub struct ItemMark;
pub struct GroupMark;

// MARK: TimelineInsert
/// For type `T` that implements [`RanimItem`],
///
/// `T` and `impl IntoIterator<Item = T>` can be inserted into the timeline.
/// This is accomplished by two implementations of this trait, with different `Mark` type:
/// - [`ItemMark`]: Insert a `T`, returns [`Rabject<T>`]
/// - [`GroupMark`]: Insert an [`IntoIterator<Item = T>`], returns [`Group<Rabject<T>>`]
pub trait TimelinePin<Mark> {
    type Output;
    fn pin(self, timeline: &RanimTimeline) -> Self::Output;
}

impl<T> TimelinePin<ItemMark> for T
where
    T: Into<Timeline> + Clone,
{
    type Output = PinnedItem<T>;
    fn pin(self, timeline: &RanimTimeline) -> Self::Output {
        let pinned_item = PinnedItem::new(self);
        // trace!("pin item: {}", pinned_item.id());
        timeline._insert_timeline(pinned_item.id(), pinned_item.data.clone().into());
        pinned_item
    }
}

impl<E, T> TimelinePin<GroupMark> for T
where
    E: TimelinePin<ItemMark>,
    T: IntoIterator<Item = E>,
{
    type Output = Group<E::Output>;
    fn pin(self, timeline: &RanimTimeline) -> Self::Output {
        self.into_iter()
            .map(|rabject| rabject.pin(timeline))
            .collect()
    }
}

pub trait TimelineUnpin<Mark> {
    type Output;
    fn unpin(self, timeline: &RanimTimeline) -> Self::Output;
}

impl<T> TimelineUnpin<ItemMark> for PinnedItem<T>
where
    T: Into<Timeline> + Clone,
{
    type Output = T;
    fn unpin(self, timeline: &RanimTimeline) -> Self::Output {
        timeline._hide(self.id());
        self.data
    }
}

impl<E, T> TimelineUnpin<GroupMark> for T
where
    E: TimelineUnpin<ItemMark>,
    T: IntoIterator<Item = E>,
{
    type Output = Group<E::Output>;
    fn unpin(self, timeline: &RanimTimeline) -> Self::Output {
        self.into_iter()
            .map(|rabject| rabject.unpin(timeline))
            .collect()
    }
}

pub trait TimelineAnim<Mark> {
    type Output;
    fn schedule(self, timeline: &RanimTimeline) -> (Self::Output, f64);
}

// For AnimationSpan, we first insert it, then play and hide the anim
impl<T: Into<Timeline> + Clone + 'static> TimelineAnim<ItemMark> for AnimationSpan<T> {
    type Output = T;
    fn schedule(self, timeline: &RanimTimeline) -> (Self::Output, f64) {
        let cur_time = timeline.cur_sec();
        let duration = self.span_len();
        // trace!(
        //     "[TimelineAnim::schedule] cur: {}, duration: {}",
        //     cur_time, duration
        // );

        let item = self.eval_alpha(0.0).into_owned();
        let rabject = timeline.pin(item);

        let res = self.eval_alpha(1.0).into_owned();
        timeline._schedule(rabject.id(), self);
        (res, cur_time + duration)
    }
}

impl<T: TimelineAnim<ItemMark>, I> TimelineAnim<GroupMark> for I
where
    I: IntoIterator<Item = T>,
{
    type Output = Group<T::Output>;
    fn schedule(self, timeline: &RanimTimeline) -> (Self::Output, f64) {
        let (results, end_times): (Vec<_>, Vec<_>) =
            self.into_iter().map(|item| item.schedule(timeline)).unzip();
        (
            Group(results),
            end_times
                .into_iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap(),
        )
    }
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

impl From<CameraFrame> for Timeline {
    fn from(value: CameraFrame) -> Self {
        let timeline = RabjectTimeline::new(value);
        Timeline::CameraFrame(Box::new(timeline))
    }
}

impl<T: VisualItem + Clone + 'static> From<T> for Timeline {
    fn from(value: T) -> Self {
        let timeline = RabjectTimeline::new(value);
        Timeline::VisualItem(Box::new(timeline))
    }
}

impl Timeline {
    pub fn as_timeline(&self) -> &dyn TimelineTrait {
        match self {
            Timeline::CameraFrame(timeline) => timeline.as_timeline(),
            Timeline::VisualItem(timeline) => timeline.as_timeline(),
        }
    }
    pub fn as_timeline_mut(&mut self) -> &mut dyn TimelineTrait {
        match self {
            Timeline::CameraFrame(timeline) => timeline.as_timeline_mut(),
            Timeline::VisualItem(timeline) => timeline.as_timeline_mut(),
        }
    }
    pub fn as_any(&self) -> &dyn Any {
        match self {
            Timeline::CameraFrame(timeline) => timeline.as_any(),
            Timeline::VisualItem(timeline) => timeline.as_any(),
        }
    }
    pub fn as_any_mut(&mut self) -> &mut dyn Any {
        match self {
            Timeline::CameraFrame(timeline) => timeline.as_any_mut(),
            Timeline::VisualItem(timeline) => timeline.as_any_mut(),
        }
    }
}

// MARK: RanimTimeline
/// The main struct that offers the ranim's API, and encodes animations
/// The rabjects insert to it will hold a reference to it, so it has interior mutability
#[derive(Default)]
pub struct RanimTimeline {
    // Timeline<CameraFrame> or Timeline<Item>
    timelines: RefCell<Vec<(usize, Timeline)>>,
    cur_secs: RefCell<f64>,
    time_marks: RefCell<Vec<(f64, TimeMark)>>,
}

impl RanimTimeline {
    pub(crate) fn seal(&self) {
        self.timelines
            .borrow_mut()
            .iter_mut()
            .for_each(|(_, timeline)| {
                timeline.as_timeline_mut().hide();
            });
        // info!("Timeline sealed");
        // info!("{}", self.cur_sec());
        // info!("{}", self.max_elapsed_secs());
    }
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cur_sec(&self) -> f64 {
        *self.cur_secs.borrow()
    }

    pub fn max_elapsed_secs(&self) -> f64 {
        self.timelines
            .borrow()
            .iter()
            .map(|(_, timeline)| timeline.as_timeline().elapsed_secs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }

    /// Use pin to pin anitem into the timeline
    pub fn pin<Mark, T: TimelinePin<Mark>>(&self, item: T) -> T::Output {
        item.pin(self)
    }

    pub fn unpin<Mark, T: TimelineUnpin<Mark>>(&self, item: T) -> T::Output {
        item.unpin(self)
    }

    fn _insert_timeline(&self, id: usize, mut timeline: Timeline) {
        let mut timelines = self.timelines.borrow_mut();

        {
            let timeline = timeline.as_timeline_mut();
            timeline.set_start_sec(self.cur_sec());
            // if max_elapsed_secs != 0.0 && timeline.elapsed_secs() < max_elapsed_secs {
            //     timeline.append_blank(*self.max_elapsed_secs.borrow());
            // }
            // trace!(
            //     "insert timeline, id: {}, cur_sec: {}, show_secs: {:?}",
            //     id,
            //     timeline.cur_sec(),
            //     timeline.show_secs()
            // );
        }
        timelines.push((id, timeline));
    }

    /// Forward all rabjects' timeline by the given seconds
    pub fn forward(&self, secs: f64) -> &Self {
        // println!("forward, sec: {}", secs);
        self.timelines
            .borrow_mut()
            .iter_mut()
            .for_each(|(_, timeline)| {
                timeline.as_timeline_mut().forward(secs);
            });
        *self.cur_secs.borrow_mut() += secs;
        self
    }
    pub fn forward_to(&self, target_sec: f64) -> &Self {
        // println!("forward_to, target_sec: {}", target_sec);
        self.timelines
            .borrow_mut()
            .iter_mut()
            .for_each(|(_, timeline)| {
                timeline.as_timeline_mut().forward_to(target_sec);
            });
        *self.cur_secs.borrow_mut() = target_sec;
        self
    }

    /// Show a rabject
    ///
    /// [`RanimTimeline::forward`] after this will encode timeline's static rabject state into the timeline
    pub fn show<T>(&self, rabject: &PinnedItem<T>) {
        self._show(rabject.id());
    }
    fn _show(&self, id: usize) {
        self.timelines
            .borrow_mut()
            .iter_mut()
            .find(|(_id, _)| *_id == id)
            .unwrap()
            .1
            .as_timeline_mut()
            .show();
    }

    // TODO: make this better
    /// Remove a rabject
    ///
    /// [`RanimTimeline::forward`] after this will encode blank into the timeline
    pub fn remove<T>(&self, rabject: PinnedItem<T>) {
        self._hide(rabject.id());
    }
    /// Hide a rabject
    ///
    /// [`RanimTimeline::forward`] after this will encode blank into the timeline
    pub fn hide<T>(&self, rabject: &PinnedItem<T>) {
        self._hide(rabject.id());
    }
    fn _hide(&self, id: usize) {
        self.timelines
            .borrow_mut()
            .iter_mut()
            .find(|(_id, _)| *_id == id)
            .unwrap()
            .1
            .as_timeline_mut()
            .hide();
    }

    /// Sync all rabjects' timeline to the max elapsed seconds
    pub fn sync(&self) -> &Self {
        let max_elapsed_secs = self.max_elapsed_secs();

        let mut timelines = self.timelines.borrow_mut();
        timelines.iter_mut().for_each(|(_, timeline)| {
            timeline.as_timeline_mut().forward_to(max_elapsed_secs);
        });
        self
    }

    /// Push an animation into the timeline
    ///
    /// Note that this won't apply the animation effect to rabject's data,
    /// to apply the animation effect use [`AnimSchedule::apply`]
    pub fn play<Mark, T: TimelineAnim<Mark>>(&self, anim: T) -> T::Output {
        let (res, end_sec) = self.schedule(anim);
        // trace!("play, end_sec: {}", end_sec);
        self.forward_to(end_sec);
        res
    }

    // pub fn play_and<Mark, T: TimelineAnim<Mark>, F, O>(&self, anim: T, and: F) -> O
    // where F: FnOnce(&RanimTimeline, T::Output) -> O {
    //     let (res, end_sec) = anim.schedule(self);
    //     self.forward_to(end_sec);
    //     res
    // }

    /// Push an animation into the timeline while not forwarding the timeline
    ///
    /// It will return the animation result and the end time of the animation
    pub fn schedule<Mark, T: TimelineAnim<Mark>>(&self, anim: T) -> (T::Output, f64) {
        anim.schedule(self)
    }

    // pub fn play_and_hide<'r, T, I>(&self, anim_schedules: I) -> &Self
    // where
    //     T: Into<Timeline> + Clone + 'static,
    //     I: IntoIterator<Item = AnimSchedule<T>>,
    // {
    //     anim_schedules
    //         .into_iter()
    //         .for_each(|AnimSchedule { id, anim }| {
    //             self._play(id, anim);
    //             self._hide(id);
    //         });
    //     self
    // }
    fn _schedule<T: Into<Timeline> + Clone + 'static>(
        &self,
        id: usize,
        anim: AnimationSpan<T>,
    ) -> &Self {
        // trace!("[RanimTimeline::_schedule] schedule {:?} on {:?}", anim, id);
        let mut timelines = self.timelines.borrow_mut();

        let timeline = timelines
            .iter_mut()
            .find(|(_id, _)| *_id == id)
            .unwrap()
            .1
            .as_any_mut()
            .downcast_mut::<RabjectTimeline<T>>()
            .unwrap();

        timeline.schedule(anim);
        self
    }

    pub fn insert_time_mark(&self, sec: f64, time_mark: TimeMark) {
        self.time_marks.borrow_mut().push((sec, time_mark));
    }
    pub fn time_marks(&self) -> Vec<(f64, TimeMark)> {
        self.time_marks.borrow().clone()
    }
    pub fn eval_sec(&self, local_sec: f64) -> TimelineEvalResult {
        self.eval_alpha(local_sec / self.cur_sec())
    }

    pub fn eval_alpha(&self, alpha: f64) -> TimelineEvalResult {
        let timelines = self.timelines.borrow_mut();

        let mut items = Vec::with_capacity(timelines.len());

        let mut camera_frame = None::<(EvalResult<CameraFrame>, usize)>;
        timelines.iter().for_each(|(id, timeline)| match timeline {
            Timeline::CameraFrame(timeline) => {
                let timeline = timeline
                    .as_any()
                    .downcast_ref::<RabjectTimeline<CameraFrame>>()
                    .unwrap();
                camera_frame = timeline.eval_alpha(alpha)
            }
            Timeline::VisualItem(timeline) => {
                if let Some((res, idx)) = timeline.eval_alpha(alpha) {
                    items.push((*id, res, idx));
                }
            }
        });
        // println!("alpha: {}, items: {}", alpha, items.len());

        TimelineEvalResult {
            camera_frame: camera_frame.unwrap(),
            visual_items: items,
        }
    }

    pub fn get_timeline_infos(&self) -> Vec<RabjectTimelineInfo> {
        // const MAX_TIMELINE_CNT: usize = 100;
        self.timelines
            .borrow()
            .iter()
            // .take(MAX_TIMELINE_CNT)
            .map(|(id, timeline)| RabjectTimelineInfo {
                id: *id,
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

impl Debug for RanimTimeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Timeline {:?}: {} timelines\n",
            Duration::from_secs_f64(self.cur_sec()),
            self.timelines.borrow().len()
        ))?;
        Ok(())
    }
}

// MARK: TimelineTrait
pub trait AnyTimelineTrait: TimelineTrait + Any {
    fn as_timeline(&self) -> &dyn TimelineTrait;
    fn as_timeline_mut(&mut self) -> &mut dyn TimelineTrait;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: TimelineTrait + Any> AnyTimelineTrait for T {
    fn as_timeline(&self) -> &dyn TimelineTrait {
        self
    }
    fn as_timeline_mut(&mut self) -> &mut dyn TimelineTrait {
        self
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub trait AnyVisualItemTimelineTrait: VisualItemTimelineTrait + Any {
    fn as_timeline(&self) -> &dyn TimelineTrait;
    fn as_timeline_mut(&mut self) -> &mut dyn TimelineTrait;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: VisualItemTimelineTrait + Any> AnyVisualItemTimelineTrait for T {
    fn as_timeline(&self) -> &dyn TimelineTrait {
        self
    }
    fn as_timeline_mut(&mut self) -> &mut dyn TimelineTrait {
        self
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub trait VisualItemTimelineTrait: TimelineTrait {
    fn eval_alpha(&self, alpha: f64) -> Option<(EvalResult<Box<dyn VisualItem>>, usize)>;
}

impl<T: Clone + VisualItem + 'static> VisualItemTimelineTrait for RabjectTimeline<T> {
    fn eval_alpha(&self, alpha: f64) -> Option<(EvalResult<Box<dyn VisualItem>>, usize)> {
        let (item, idx) = self.eval_alpha(alpha)?;
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
    fn set_start_sec(&mut self, start_sec: f64);
    fn show_secs(&self) -> &Vec<f64>;
}

// MARK: RabjectTimeline
/// A timeline struct that encodes the animation of the type `T`
pub struct RabjectTimeline<T> {
    type_name: String,
    cur_sec: f64,
    animations: Vec<AnimationSpan<T>>,
    show_secs: Vec<f64>,
}

impl<T> TimelineTrait for RabjectTimeline<T> {
    fn elapsed_secs(&self) -> f64 {
        self.show_secs.last().copied().unwrap_or(0.0)
    }
    fn cur_sec(&self) -> f64 {
        self.cur_sec
    }
    fn show_secs(&self) -> &Vec<f64> {
        &self.show_secs
    }
    fn show(&mut self) {
        if self.show_secs.len() % 2 == 0 {
            // trace!("!!! push {}", self.cur_sec());
            self.show_secs.push(self.cur_sec());
        }
    }
    fn hide(&mut self) {
        if self.show_secs.len() % 2 != 0 {
            // trace!("!!! push {}", self.cur_sec());
            self.show_secs.push(self.cur_sec());
        }
    }
    fn forward(&mut self, duration_secs: f64) {
        // trace!(
        //     "[{}] cur: {}, forward {} secs",
        //     self.type_name,
        //     self.cur_sec(),
        //     duration_secs
        // );
        self.cur_sec += duration_secs;
        // if self.is_showing() {
        //     self.append_freeze(duration_secs);
        // } else {
        //     self.append_blank(duration_secs);
        // }
    }
    // fn append_blank(&mut self, duration_secs: f64) {
    //     let end_sec = self.end_secs.last().copied().unwrap_or(0.0) + duration_secs;
    //     self.animations.push(None);
    //     self.end_secs.push(end_sec);
    // }
    // fn append_freeze(&mut self, duration_secs: f64) {
    //     let end_sec = self.end_secs.last().copied().unwrap_or(0.0) + duration_secs;
    //     self.animations.push(
    //         self.forward_static_state
    //             .as_ref()
    //             .map(|state| AnimationSpan::from_evaluator(Evaluator::Static(state.clone()))),
    //     );
    //     self.end_secs.push(end_sec);
    // }
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
    fn set_start_sec(&mut self, start_sec: f64) {
        assert_eq!(self.show_secs.len(), 1);
        self.cur_sec = start_sec;
        self.show_secs[0] = start_sec;
    }
}

impl<T: 'static> RabjectTimeline<T> {
    /// Create a new timeline with the initial state
    pub fn new(initial_state: T) -> Self {
        Self {
            type_name: std::any::type_name::<T>().to_string(),
            animations: vec![AnimationSpan::from_evaluator(Evaluator::Static(Rc::new(
                initial_state,
            )))],
            cur_sec: 0.0,
            show_secs: vec![0.0],
        }
    }
}

impl<T> RabjectTimeline<T> {
    fn is_showing(&self) -> bool {
        self.show_secs
            .chunks(2)
            .enumerate()
            .any(|(idx, show_secs)| {
                let start = show_secs.first().cloned().unwrap();
                let end = show_secs.get(1).cloned().unwrap_or(self.cur_sec());
                start <= self.cur_sec
                    && (self.cur_sec < end
                        || self.cur_sec == end && idx == self.animations.len() - 1)
            })
    }
    fn schedule(&mut self, anim: AnimationSpan<T>) {
        // trace!(
        //     "[RabjectTimeline::schedule] schedule {:?}, cur: {}, show_secs: {:?}",
        //     anim,
        //     self.cur_sec(),
        //     self.show_secs
        // );
        if self.is_showing() {
            self.show_secs.push(self.cur_sec);
        }
        self.show_secs.push(self.cur_sec);
        self.show_secs.push(self.cur_sec + anim.span_len());
        self.animations.push(anim);
        // trace!(
        //     "[RabjectTimeline::schedule] schedule, cur: {}, show_secs: {:?}",
        //     self.cur_sec(),
        //     self.show_secs
        // );
    }
    pub fn eval_alpha(&self, alpha: f64) -> Option<(EvalResult<T>, usize)> {
        if self.animations.is_empty() {
            return None;
        }

        let alpha = alpha.clamp(0.0, 1.0);
        let target_sec = alpha * self.cur_sec();
        // trace!(
        //     "[RabjectTimeline::eval_alpha alpha: {}, target_sec: {}",
        //     alpha, target_sec
        // );
        // trace!("cur: {}, show_secs {:?}", self.cur_sec(), self.show_secs);
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
