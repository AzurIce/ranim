use log::trace;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    animation::{AnimSchedule, AnimationSpan, EvalResult, Evaluator},
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
        let cur_time = timeline.duration_secs();
        let duration = self.span_len();

        let item = self.eval_alpha(0.0).into_owned();
        let rabject = timeline.pin(item);

        let res = self.eval_alpha(1.0).into_owned();
        timeline._play(rabject.id(), self);
        timeline.hide(&rabject);
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
    max_elapsed_secs: RefCell<f64>,
    time_marks: RefCell<Vec<(f64, TimeMark)>>,
}

impl RanimTimeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn duration_secs(&self) -> f64 {
        *self.max_elapsed_secs.borrow()
    }

    /// Use pin to pin anitem into the timeline
    pub fn pin<Mark, T: TimelinePin<Mark>>(&self, item: T) -> T::Output {
        item.pin(self)
    }

    pub fn unpin<Mark, T: TimelineUnpin<Mark>>(&self, item: T) -> T::Output {
        item.unpin(self)
    }

    // pub fn insert_and<Mark, T: TimelineItem<Mark>, F>(&self, item: T, f: F)
    // where
    //     F: FnOnce(&Self, &mut T::Inserted),
    // {
    //     let mut inserted = item.insert_into_timeline(self);
    //     f(self, &mut inserted);
    //     inserted
    // }

    fn _insert_timeline(&self, id: usize, mut timeline: Timeline) {
        let mut timelines = self.timelines.borrow_mut();

        let max_elapsed_secs = *self.max_elapsed_secs.borrow();
        {
            let timeline = timeline.as_timeline_mut();
            if max_elapsed_secs != 0.0 && timeline.elapsed_secs() < max_elapsed_secs {
                timeline.append_blank(*self.max_elapsed_secs.borrow());
            }
        }
        timelines.push((id, timeline));
    }

    /// Update the static state of a rabject's timeline
    pub fn update<T: Into<Timeline> + Clone + 'static>(&self, rabject: &PinnedItem<T>) {
        self._update(rabject.id(), rabject.data.clone());
    }
    pub fn _update<T: Into<Timeline> + Clone + 'static>(&self, id: usize, data: T) {
        let mut timelines = self.timelines.borrow_mut();
        let timeline = timelines
            .iter_mut()
            .find(|(_id, _)| *_id == id)
            .unwrap()
            .1
            .as_any_mut()
            .downcast_mut::<RabjectTimeline<T>>()
            .unwrap();
        timeline.update_static_state(Some(Rc::new(data)));
    }
    /// Forward all rabjects' timeline by the given seconds
    pub fn forward(&self, secs: f64) -> &Self {
        self.timelines
            .borrow_mut()
            .iter_mut()
            .for_each(|(_, timeline)| {
                timeline.as_timeline_mut().forward(secs);
            });
        *self.max_elapsed_secs.borrow_mut() += secs;
        self
    }
    pub fn forward_to(&self, target_sec: f64) -> &Self {
        let duration = target_sec - self.duration_secs();
        if duration > 0.0 {
            self.forward(duration);
        }
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
        let mut timelines = self.timelines.borrow_mut();
        let max_elapsed_secs = self.max_elapsed_secs.borrow();
        timelines.iter_mut().for_each(|(id, timeline)| {
            // println!("{}, {}", timeline.elapsed_secs(), *max_elapsed_secs);
            let elapsed_secs = timeline.as_timeline().elapsed_secs();
            if elapsed_secs < *max_elapsed_secs {
                timeline
                    .as_timeline_mut()
                    .forward(*max_elapsed_secs - elapsed_secs);
            }
        });
        self
    }

    /// Push an animation into the timeline
    ///
    /// Note that this won't apply the animation effect to rabject's data,
    /// to apply the animation effect use [`AnimSchedule::apply`]
    pub fn play<Mark, T: TimelineAnim<Mark>>(&self, anim: T) -> T::Output {
        let (res, end_sec) = anim.schedule(self);
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
        let res = anim.schedule(self);
        res
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
    fn _play<T: Into<Timeline> + Clone + 'static>(
        &self,
        id: usize,
        anim: AnimationSpan<T>,
    ) -> &Self {
        trace!("play {:?} on {:?}", anim, id);
        let mut timelines = self.timelines.borrow_mut();

        let timeline = timelines
            .iter_mut()
            .find(|(_id, _)| *_id == id)
            .unwrap()
            .1
            .as_any_mut()
            .downcast_mut::<RabjectTimeline<T>>()
            .unwrap();

        let mut max_duration = self.max_elapsed_secs.borrow_mut();
        timeline.append_anim(anim);
        *max_duration = max_duration.max(timeline.duration_secs());
        self
    }

    pub fn insert_time_mark(&self, sec: f64, time_mark: TimeMark) {
        self.time_marks.borrow_mut().push((sec, time_mark));
    }
    pub fn time_marks(&self) -> Vec<(f64, TimeMark)> {
        self.time_marks.borrow().clone()
    }
    pub fn eval_sec(&self, local_sec: f64) -> TimelineEvalResult {
        self.eval_alpha(local_sec / *self.max_elapsed_secs.borrow())
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
            Duration::from_secs_f64(self.duration_secs()),
            self.timelines.borrow().len()
        ))?;
        Ok(())
    }
}

// MARK: RabjectTimeline

/// A timeline struct that encodes the animation of the type `T`
pub struct RabjectTimeline<T> {
    type_name: String,
    forward_static_state: Option<Rc<T>>,
    animations: Vec<Option<AnimationSpan<T>>>,
    end_secs: Vec<f64>,
    // Encoding states
    is_showing: bool,
}

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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AnimationInfo {
    pub anim_name: String,
    pub start_sec: f64,
    pub end_sec: f64,
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

pub trait TimelineTrait {
    fn elapsed_secs(&self) -> f64;
    fn forward(&mut self, duration_secs: f64);
    fn append_blank(&mut self, duration_secs: f64);
    fn append_freeze(&mut self, duration_secs: f64);
    fn show(&mut self);
    fn hide(&mut self);
    fn get_animation_infos(&self) -> Vec<AnimationInfo>;
    fn type_name(&self) -> &str;
}

impl<T: 'static> TimelineTrait for RabjectTimeline<T> {
    fn elapsed_secs(&self) -> f64 {
        self.end_secs.last().cloned().unwrap_or(0.0)
    }
    fn show(&mut self) {
        self.is_showing = true;
    }
    fn hide(&mut self) {
        self.is_showing = false;
    }
    fn forward(&mut self, duration_secs: f64) {
        if self.is_showing {
            self.append_freeze(duration_secs);
        } else {
            self.append_blank(duration_secs);
        }
    }
    fn append_blank(&mut self, duration_secs: f64) {
        let end_sec = self.end_secs.last().copied().unwrap_or(0.0) + duration_secs;
        self.animations.push(None);
        self.end_secs.push(end_sec);
    }
    fn append_freeze(&mut self, duration_secs: f64) {
        let end_sec = self.end_secs.last().copied().unwrap_or(0.0) + duration_secs;
        self.animations.push(
            self.forward_static_state
                .as_ref()
                .map(|state| AnimationSpan::from_evaluator(Evaluator::Static(state.clone()))),
        );
        self.end_secs.push(end_sec);
    }
    fn get_animation_infos(&self) -> Vec<AnimationInfo> {
        // const MAX_INFO_CNT: usize = 100;
        self.animations
            .iter()
            .enumerate()
            .filter_map(|(idx, anim)| {
                anim.as_ref().map(|anim| AnimationInfo {
                    anim_name: anim.type_name.clone(),
                    start_sec: if idx == 0 {
                        0.0
                    } else {
                        self.end_secs.get(idx - 1).cloned().unwrap()
                    } + anim.padding.0,
                    end_sec: self.end_secs[idx] - anim.padding.1,
                })
            })
            // .take(MAX_INFO_CNT)
            .collect()
    }
    fn type_name(&self) -> &str {
        &self.type_name
    }
}

impl<T: 'static> RabjectTimeline<T> {
    pub fn new(initial_state: T) -> Self {
        Self {
            type_name: std::any::type_name::<T>().to_string(),
            forward_static_state: Some(Rc::new(initial_state)),
            animations: Vec::new(),
            end_secs: Vec::new(),
            is_showing: true,
        }
    }
}

impl<T> RabjectTimeline<T> {
    fn update_static_state(&mut self, static_state: Option<Rc<T>>) {
        self.forward_static_state = static_state;
    }
    fn append_anim(&mut self, anim: AnimationSpan<T>) {
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
    pub fn duration_secs(&self) -> f64 {
        self.end_secs.last().cloned().unwrap_or(0.0)
    }
    pub fn eval_alpha(&self, alpha: f64) -> Option<(EvalResult<T>, usize)> {
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
