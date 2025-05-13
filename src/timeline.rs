use log::trace;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    animation::{AnimSchedule, AnimationSpan, EvalResult, Evaluator},
    items::{Rabject, camera_frame::CameraFrame, group::Group},
    render::primitives::RenderableItem,
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
    pub items: Vec<(usize, EvalResult<Box<dyn RenderableItem>>, usize)>,
}

// MARK: TimelineInsert
/// For type `T` that implements [`RanimItem`],
///
/// `T` and `impl IntoIterator<Item = T>` can be inserted into the timeline.
/// This is accomplished by two implementations of this trait, with different `Mark` type:
/// - [`ItemMark`]: Insert a `T`, returns [`Rabject<T>`]
/// - [`GroupMark`]: Insert an [`IntoIterator<Item = T>`], returns [`Group<Rabject<T>>`]
pub trait TimelineItem<Mark> {
    type Inserted;
    fn insert_into_timeline(self, timeline: &RanimTimeline) -> Self::Inserted;
}

// MARK: TimelineItem
pub struct ItemMark;
pub struct GroupMark;

impl<T> TimelineItem<ItemMark> for T
where
    T: RanimItem + 'static,
{
    type Inserted = Rabject<T>;
    fn insert_into_timeline(self, timeline: &RanimTimeline) -> Self::Inserted {
        RanimItem::insert_into_timeline(self, timeline)
    }
}

impl<E, T> TimelineItem<GroupMark> for T
where
    E: RanimItem + 'static,
    T: IntoIterator<Item = E>,
{
    type Inserted = Group<Rabject<E>>;
    fn insert_into_timeline(self, timeline: &RanimTimeline) -> Self::Inserted {
        self.into_iter()
            .map(|item| RanimItem::insert_into_timeline(item, timeline))
            .collect()
    }
}

/// An item that can be inserted into ranim's timeline
///
/// The item `T` will be inserted into a [`RabjectTimeline<T>`],
/// and the [`RabjectTimeline<T>`] will be inserted into a [`RanimTimeline`] with type erased.
///
/// For now, there are two fixed types of [`RanimItem`], and they will be erased to different types:
/// - [`CameraFrame`]: A camera frame.
///   It will be erased to [`Timeline::CameraFrame`], which has a boxed [`AnyTimelineTrait`] in it.
/// - [`RenderableItem`]: A renderable item
///   It will be erased to [`Timeline::RenderableItem`], which has a boxed [`RenderableTimelineTrait`] in it.
pub trait RanimItem {
    fn insert_into_timeline(self, timeline: &RanimTimeline) -> Rabject<Self>
    where
        Self: Sized;
}

impl<T: RenderableItem + Clone + 'static> RanimItem for T {
    fn insert_into_timeline(self, ranim_timeline: &RanimTimeline) -> Rabject<Self> {
        let timeline = RabjectTimeline::new(self.clone());
        let timeline = Timeline::RenderableItem(Box::new(timeline));
        Rabject {
            id: ranim_timeline.insert_timeline(timeline),
            data: self,
        }
    }
}

impl RanimItem for CameraFrame {
    fn insert_into_timeline(self, ranim_timeline: &RanimTimeline) -> Rabject<Self> {
        let timeline = RabjectTimeline::new(self.clone());
        let timeline = Timeline::CameraFrame(Box::new(timeline));
        Rabject {
            id: ranim_timeline.insert_timeline(timeline),
            data: self,
        }
    }
}

pub enum Timeline {
    CameraFrame(Box<dyn AnyTimelineTrait>),
    RenderableItem(Box<dyn AnyRenderableTimelineTrait>),
}

impl Timeline {
    pub fn as_timeline(&self) -> &dyn TimelineTrait {
        match self {
            Timeline::CameraFrame(timeline) => timeline.as_timeline(),
            Timeline::RenderableItem(timeline) => timeline.as_timeline(),
        }
    }
    pub fn as_timeline_mut(&mut self) -> &mut dyn TimelineTrait {
        match self {
            Timeline::CameraFrame(timeline) => timeline.as_timeline_mut(),
            Timeline::RenderableItem(timeline) => timeline.as_timeline_mut(),
        }
    }
    pub fn as_any(&self) -> &dyn Any {
        match self {
            Timeline::CameraFrame(timeline) => timeline.as_any(),
            Timeline::RenderableItem(timeline) => timeline.as_any(),
        }
    }
    pub fn as_any_mut(&mut self) -> &mut dyn Any {
        match self {
            Timeline::CameraFrame(timeline) => timeline.as_any_mut(),
            Timeline::RenderableItem(timeline) => timeline.as_any_mut(),
        }
    }
}

// MARK: RanimTimeline
/// The main struct that offers the ranim's API, and encodes animations
/// The rabjects insert to it will hold a reference to it, so it has interior mutability
#[derive(Default)]
pub struct RanimTimeline {
    // Timeline<CameraFrame> or Timeline<Item>
    timelines: RefCell<Vec<Timeline>>,
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

    /// Push an animation into the timeline
    ///
    /// Note that this won't apply the animation effect to rabject's data,
    /// to apply the animation effect use [`AnimSchedule::apply`]
    pub fn play<'r, T, I>(&self, anim_schedules: I) -> &Self
    where
        T: RanimItem + Clone + 'static,
        I: IntoIterator<Item = AnimSchedule<'r, T>>,
    {
        anim_schedules.into_iter().for_each(|anim_schedule| {
            self._play(anim_schedule);
        });
        self
    }

    pub fn insert<Mark, T: TimelineItem<Mark>>(&self, item: T) -> T::Inserted {
        item.insert_into_timeline(self)
    }

    fn insert_timeline(&self, mut timeline: Timeline) -> usize {
        let mut timelines = self.timelines.borrow_mut();

        let id = timelines.len();
        let max_elapsed_secs = *self.max_elapsed_secs.borrow();
        {
            let timeline = timeline.as_timeline_mut();
            if max_elapsed_secs != 0.0 && timeline.elapsed_secs() < max_elapsed_secs {
                timeline.append_blank(*self.max_elapsed_secs.borrow());
            }
        }
        timelines.push(timeline);
        id
    }

    /// Update the static state of a rabject's timeline
    pub fn update<T: RanimItem + Clone + 'static>(&self, rabject: &Rabject<T>) {
        let mut timelines = self.timelines.borrow_mut();
        let timeline = timelines
            .get_mut(rabject.id)
            .unwrap()
            .as_any_mut()
            .downcast_mut::<RabjectTimeline<T>>()
            .unwrap();
        timeline.update_static_state(Some(Rc::new(rabject.data.clone())));
    }
    /// Forward all rabjects' timeline by the given seconds
    pub fn forward(&self, secs: f64) -> &Self {
        self.timelines.borrow_mut().iter_mut().for_each(|timeline| {
            timeline.as_timeline_mut().forward(secs);
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
            .as_timeline_mut()
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
            .as_timeline_mut()
            .hide();
    }

    // TODO: make this better
    /// Remove a rabject
    ///
    /// [`RanimTimeline::forward`] after this will encode blank into the timeline
    pub fn remove<T>(&self, rabject: Rabject<T>) {
        self.timelines
            .borrow_mut()
            .get_mut(rabject.id)
            .unwrap()
            .as_timeline_mut()
            .hide();
    }

    /// Sync all rabjects' timeline to the max elapsed seconds
    pub fn sync(&self) -> &Self {
        let mut timelines = self.timelines.borrow_mut();
        let max_elapsed_secs = self.max_elapsed_secs.borrow();
        timelines.iter_mut().for_each(|timeline| {
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

    fn _play<T: RanimItem + Clone + 'static>(&self, anim_schedule: AnimSchedule<T>) -> &Self {
        trace!("play {:?}", anim_schedule);
        let mut timelines = self.timelines.borrow_mut();
        let AnimSchedule { rabject, anim } = anim_schedule;

        let timeline = timelines
            .get_mut(rabject.id)
            .unwrap()
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
        self.eval_alpha_new(local_sec / *self.max_elapsed_secs.borrow())
    }

    pub fn eval_alpha_new(&self, alpha: f64) -> TimelineEvalResult {
        let timelines = self.timelines.borrow_mut();

        let mut items = Vec::with_capacity(timelines.len());

        let mut camera_frame = None::<(EvalResult<CameraFrame>, usize)>;
        timelines
            .iter()
            .enumerate()
            .for_each(|(id, timeline)| match timeline {
                Timeline::CameraFrame(timeline) => {
                    let timeline = timeline
                        .as_any()
                        .downcast_ref::<RabjectTimeline<CameraFrame>>()
                        .unwrap();
                    camera_frame = timeline.eval_alpha(alpha)
                }
                Timeline::RenderableItem(timeline) => {
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

    pub fn get_timeline_infos(&self) -> Vec<RabjectTimelineInfo> {
        // const MAX_TIMELINE_CNT: usize = 100;
        self.timelines
            .borrow()
            .iter()
            .enumerate()
            // .take(MAX_TIMELINE_CNT)
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

pub trait AnyRenderableTimelineTrait: RenderableTimelineTrait + Any {
    fn as_timeline(&self) -> &dyn TimelineTrait;
    fn as_timeline_mut(&mut self) -> &mut dyn TimelineTrait;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: RenderableTimelineTrait + Any> AnyRenderableTimelineTrait for T {
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

pub trait RenderableTimelineTrait: TimelineTrait {
    fn eval_alpha(&self, alpha: f64) -> Option<(EvalResult<Box<dyn RenderableItem>>, usize)>;
}

impl<T: Clone + RenderableItem + 'static> RenderableTimelineTrait for RabjectTimeline<T> {
    fn eval_alpha(&self, alpha: f64) -> Option<(EvalResult<Box<dyn RenderableItem>>, usize)> {
        let (item, idx) = self.eval_alpha(alpha)?;
        let item = item.map(|item| Box::new(item) as Box<dyn RenderableItem>);
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
