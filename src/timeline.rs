use itertools::Itertools;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    animation::{AnimationSpan, EvalResult, Evaluator},
    items::{ItemId, VisualItem, camera_frame::CameraFrame},
};
use std::fmt::Debug;
use std::{any::Any, sync::Arc};

/// TimeMark
#[derive(Debug, Clone)]
pub enum TimeMark {
    /// Capture a picture with a name
    Capture(String),
}

/// The evaluation result
///
/// This is produced from [`SealedRanimScene::eval_alpha`] or [`SealedRanimScene::eval_sec`]
#[allow(clippy::type_complexity)]
pub struct TimelineEvalResult {
    /// (`EvalResult<CameraFrame>`, `timeline idx` `animation idx`)
    pub camera_frame: (EvalResult<CameraFrame>, usize, usize),
    /// (`id`, `EvalResult<Box<dyn RenderableItem>>`, `timeline idx` `animation idx`)
    pub visual_items: Vec<(usize, EvalResult<Box<dyn VisualItem>>, usize, usize)>,
}

// MARK: RanimScene
/// The main struct that offers the ranim's API, and encodes animations
/// The rabjects insert to it will hold a reference to it, so it has interior mutability
#[derive(Default)]
pub struct RanimScene {
    // Timeline<CameraFrame> or Timeline<Item>
    timelines: Vec<ItemDynTimelines>,
    time_marks: Vec<(f64, TimeMark)>,
}

impl RanimScene {
    /// Seals the scene to [`SealedRanimScene`].
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
    /// Create a new [`RanimScene`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Use the item state to create a new [`ItemDynTimelines`] and returns the [`ItemId`].
    ///
    /// Note that, the new timeline is hidden by default, use [`ItemTimeline::forward`] and
    /// [`ItemTimeline::forward_to`] to modify the start time of the first anim, and use
    /// [`ItemTimeline::show`] to start encoding and static anim.
    pub fn insert<T: Clone + 'static>(&mut self, state: T) -> ItemId<T>
    where
        ItemTimeline<T>: Into<DynTimeline>,
    {
        self.insert_and(state, |_| {})
    }
    /// Use the item state to create a new [`ItemDynTimelines`], and call [`ItemTimeline::show`] on it.
    pub fn insert_and_show<T: Clone + 'static>(&mut self, state: T) -> ItemId<T>
    where
        ItemTimeline<T>: Into<DynTimeline>,
    {
        self.insert_and(state, |timeline| {
            timeline.show();
        })
    }
    /// Use the item state to create a new [`ItemDynTimelines`], and call `f` on it.
    pub fn insert_and<T: Clone + 'static>(
        &mut self,
        state: T,
        f: impl FnOnce(&mut ItemTimeline<T>),
    ) -> ItemId<T>
    where
        ItemTimeline<T>: Into<DynTimeline>,
    {
        let id = ItemId::alloc();
        let mut item_timeline = ItemTimeline::<T>::new(state);
        f(&mut item_timeline);
        self.timelines.push(ItemDynTimelines {
            id: *id,
            timelines: vec![item_timeline.into()],
        });
        id
    }
    /// Consumes an [`ItemId<T>`], and convert it into [`ItemId<E>`].
    ///
    /// This insert inserts an [`ItemTimeline<E>`] into the corresponding [`ItemDynTimelines`]
    pub fn map<T: Clone + 'static, E: Clone + 'static>(
        &mut self,
        item_id: ItemId<T>,
        map_fn: impl FnOnce(T) -> E,
    ) -> ItemId<E>
    where
        ItemTimeline<E>: Into<DynTimeline>,
    {
        let item_dyn_timeline = self
            .timelines
            .iter_mut()
            .find(|timeline| timeline.id == *item_id)
            .unwrap();
        item_dyn_timeline.apply_map(map_fn);
        ItemId::new(item_id.inner())
    }

    /// Get reference of all timelines in the type erased [`ItemDynTimelines`] type.
    pub fn timelines(&self) -> &[ItemDynTimelines] {
        &self.timelines
    }
    /// Get mutable reference of all timelines in the type erased [`ItemDynTimelines`] type.
    pub fn timelines_mut(&mut self) -> &mut [ItemDynTimelines] {
        &mut self.timelines
    }
    /// Get the reference of timeline(s) by the [`TimelineIndex`].
    pub fn timeline<'a, T: TimelineIndex<'a>>(&'a self, index: &T) -> T::RefOutput {
        index.timeline(self)
    }
    /// Get the mutable reference of timeline(s) by the [`TimelineIndex`].
    pub fn timeline_mut<'a, T: TimelineIndex<'a>>(&'a mut self, index: &T) -> T::MutOutput {
        index.timeline_mut(self)
    }
    /// Inserts an [`TimeMark`]
    pub fn insert_time_mark(&mut self, sec: f64, time_mark: TimeMark) {
        self.time_marks.push((sec, time_mark));
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// The information of an [`ItemDynTimelines`].
pub struct ItemDynTimelinesInfo {
    /// The inner id value of the [`ItemId`]
    pub id: usize,
    /// The animation infos.
    pub animation_infos: Vec<AnimationInfo>,
}

impl Debug for RanimScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Timeline: {} timelines", self.timelines.len()))?;
        Ok(())
    }
}

/// The sealed [`RanimScene`].
///
/// the timelines and time marks cannot be modified after sealed. And
/// once the [`RanimScene`] is sealed, it can be used for evaluating.
pub struct SealedRanimScene {
    total_secs: f64,
    timelines: Vec<ItemDynTimelines>,
    time_marks: Vec<(f64, TimeMark)>,
}

impl SealedRanimScene {
    /// Get the total seconds of the [`SealedRanimScene`].
    pub fn total_secs(&self) -> f64 {
        self.total_secs
    }
    /// Get time marks
    pub fn time_marks(&self) -> &Vec<(f64, TimeMark)> {
        &self.time_marks
    }
    /// Evaluate the state of timelines at `target_sec`
    pub fn eval_sec(&self, target_sec: f64) -> TimelineEvalResult {
        let mut items = Vec::with_capacity(self.timelines.len());

        let mut camera_frame = None::<(EvalResult<CameraFrame>, usize, usize)>;

        for timeline in &self.timelines {
            let Some((timeline_idx, res)) = timeline.eval_sec(target_sec) else {
                continue;
            };
            match res {
                DynTimelineEvalResult::CameraFrame((res, idx)) => {
                    camera_frame = Some((res, timeline_idx, idx))
                }
                DynTimelineEvalResult::VisualItem((res, idx)) => {
                    items.push((timeline.id, res, timeline_idx, idx));
                }
            }
        }

        TimelineEvalResult {
            camera_frame: camera_frame.unwrap(),
            visual_items: items,
        }
    }
    /// Evaluate the state of timelines at `alpha`
    pub fn eval_alpha(&self, alpha: f64) -> TimelineEvalResult {
        self.eval_sec(alpha * self.total_secs)
    }

    /// Get timeline infos.
    pub fn get_timeline_infos(&self) -> Vec<ItemDynTimelinesInfo> {
        // const MAX_TIMELINE_CNT: usize = 100;
        self.timelines
            .iter()
            // .take(MAX_TIMELINE_CNT)
            .map(|timeline| ItemDynTimelinesInfo {
                id: timeline.id,
                animation_infos: (timeline as &dyn TimelineFunc).get_animation_infos(),
            })
            .collect()
    }
}

// MARK: TimelineIndex
/// A trait for indexing timeline(s)
///
/// [`RanimScene::timeline`] and [`RanimScene::timeline_mut`] uses the
/// reference of [`TimelineIndex`] to index the timeline(s).
///
/// | Index Type | Output Type |
/// |------------|-------------|
/// |   `usize`    | `&ItemDynTimelines` and `&mut ItemDynTimelines` |
/// |   `ItemId<T>`    | `&ItemTimeline<T>` & and `&mut ItemTimeline<T>` |
/// |   `[&ItemId<T>; N]`    | `[&ItemTimeline<T>; N]` and `[&mut ItemTimeline<T>; N]` |
pub trait TimelineIndex<'a> {
    /// Output of [`TimelineIndex::timeline`]
    type RefOutput;
    /// Output of [`TimelineIndex::timeline_mut`]
    type MutOutput;
    /// Get the reference of timeline(s) from [`RanimScene`] by the [`TimelineIndex`].
    fn timeline(&self, timeline: &'a RanimScene) -> Self::RefOutput;
    /// Get the mutable reference of timeline(s) from [`RanimScene`] by the [`TimelineIndex`].
    fn timeline_mut(&self, timeline: &'a mut RanimScene) -> Self::MutOutput;
}

impl<'a> TimelineIndex<'a> for usize {
    type RefOutput = Option<&'a ItemDynTimelines>;
    type MutOutput = Option<&'a mut ItemDynTimelines>;
    fn timeline(&self, timeline: &'a RanimScene) -> Self::RefOutput {
        timeline
            .timelines()
            .iter()
            .find(|timeline| *self == timeline.id)
    }
    fn timeline_mut(&self, timeline: &'a mut RanimScene) -> Self::MutOutput {
        timeline
            .timelines_mut()
            .iter_mut()
            .find(|timeline| *self == timeline.id)
    }
}

impl<'a, T: 'static> TimelineIndex<'a> for ItemId<T> {
    type RefOutput = &'a ItemTimeline<T>;
    type MutOutput = &'a mut ItemTimeline<T>;
    fn timeline(&self, timeline: &'a RanimScene) -> Self::RefOutput {
        timeline
            .timelines()
            .iter()
            .find(|timeline| **self == timeline.id)
            .map(|timeline| timeline.get())
            .unwrap()
    }
    fn timeline_mut(&self, timeline: &'a mut RanimScene) -> Self::MutOutput {
        timeline
            .timelines_mut()
            .iter_mut()
            .find(|timeline| **self == timeline.id)
            .map(|timeline| timeline.get_mut())
            .unwrap()
    }
}

impl<'a, T: 'static, const N: usize> TimelineIndex<'a> for [&ItemId<T>; N] {
    type RefOutput = [&'a ItemTimeline<T>; N];
    type MutOutput = [&'a mut ItemTimeline<T>; N];
    fn timeline(&self, timeline: &'a RanimScene) -> Self::RefOutput {
        // TODO: the order is not stable
        let mut timelines = timeline
            .timelines()
            .iter()
            .filter(|timeline| self.iter().any(|id| ***id == timeline.id))
            .collect_array::<N>()
            .unwrap();
        timelines.sort_by_key(|timeline| self.iter().position(|id| ***id == timeline.id).unwrap());
        timelines.map(|timeline| timeline.get())
    }
    fn timeline_mut(&self, timeline: &'a mut RanimScene) -> Self::MutOutput {
        // TODO: the order is not stable
        let mut timelines = timeline
            .timelines_mut()
            .iter_mut()
            .filter(|timeline| self.iter().any(|id| ***id == timeline.id))
            .collect_array::<N>()
            .unwrap();
        timelines.sort_by_key(|timeline| self.iter().position(|id| ***id == timeline.id).unwrap());
        timelines.map(|timeline| timeline.get_mut())
    }
}

// MARK: TimelinesFunc
/// Functions for timelines
pub trait TimelinesFunc {
    /// Seal timelines
    fn seal(&mut self);
    /// Get the max end_sec of the timelines
    fn max_total_secs(&self) -> f64;
    /// Sync the timelines
    fn sync(&mut self);
    /// Forward all timelines by `sec`
    fn forward(&mut self, secs: f64);
    /// Forward all timelines to `target_sec`
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
/// Any + TimelineFunc
pub trait AnyTimelineFunc: TimelineFunc + Any {}
impl<T: TimelineFunc + Any> AnyTimelineFunc for T {}

/// Any + VisualItemTimelineTrait
pub trait AnyVisualItemTimelineTrait: VisualItemTimelineTrait + Any {}
impl<T: VisualItemTimelineTrait + Any> AnyVisualItemTimelineTrait for T {}

// ANCHOR: VisualItemTimelineTrait
/// A visual item timeline, which can eval to `EvalResult<Box<dyn VisualItem>>`.
///
/// This is auto implemented for `ItemTimeline<T>` where `T: Clone + VisualItem + 'static`
pub trait VisualItemTimelineTrait: TimelineFunc {
    /// Evaluate the timeline at `target_sec`
    fn eval_sec(&self, target_sec: f64) -> Option<(EvalResult<Box<dyn VisualItem>>, usize)>;
}
// ANCHOR_END: VisualItemTimelineTrait

impl<T: Clone + VisualItem + 'static> VisualItemTimelineTrait for ItemTimeline<T> {
    fn eval_sec(&self, target_sec: f64) -> Option<(EvalResult<Box<dyn VisualItem>>, usize)> {
        let (item, idx) = self.eval_sec(target_sec)?;
        let item = item.map(|item| Box::new(item) as Box<dyn VisualItem>);
        Some((item, idx))
    }
}

/// Info of an animation
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AnimationInfo {
    /// The name of the animation
    pub anim_name: String,
    /// The time range of the animation
    pub range: std::ops::Range<f64>,
}

// MARK: TimelineFunc
/// Functions for a timeline
pub trait TimelineFunc {
    /// The start sec of the timeline(the start sec of the first animation.)
    fn start_sec(&self) -> Option<f64>;
    /// The end sec of the timeline(the end sec of the last animation.)
    fn end_sec(&self) -> Option<f64>;
    /// The range of the timeline.
    fn range_sec(&self) -> Option<std::ops::Range<f64>> {
        let (Some(start), Some(end)) = (self.start_sec(), self.end_sec()) else {
            return None;
        };
        Some(start..end)
    }
    /// Seal the timeline func(submit the planning static anim)
    fn seal(&mut self);
    /// The current sec of the timeline.
    fn cur_sec(&self) -> f64;
    /// Forward the timeline by `secs`
    fn forward(&mut self, secs: f64);
    /// Forward the timeline to `target_sec`
    fn forward_to(&mut self, target_sec: f64) {
        let duration = target_sec - self.cur_sec();
        if duration > 0.0 {
            self.forward(duration);
        }
    }
    /// Show the item
    fn show(&mut self);
    /// Hide the item
    fn hide(&mut self);
    /// Get the animation infos
    fn get_animation_infos(&self) -> Vec<AnimationInfo>;
    /// The type name of the timeline
    fn type_name(&self) -> &str;
}

// ANCHOR: ItemDynTimelines
/// A item timeline which contains multiple [`DynTimeline`], so
/// that it can contains multiple [`ItemTimeline<T>`] in different type of `T`.
pub struct ItemDynTimelines {
    id: usize,
    timelines: Vec<DynTimeline>,
}
// ANCHOR_END: ItemDynTimelines

// MARK: DynTimelineEvalResult
/// The eval result of [`ItemDynTimelines`]
pub enum DynTimelineEvalResult {
    /// The eval result of [`ItemTimeline<CameraFrame>`]
    CameraFrame((EvalResult<CameraFrame>, usize)),
    /// The eval result of [`ItemTimeline<T>`] where `T` is a [`VisualItem`]
    VisualItem((EvalResult<Box<dyn VisualItem>>, usize)),
}

impl ItemDynTimelines {
    /// Evaluate the timeline at `alpha`
    pub fn eval_alpha(&self, alpha: f64) -> Option<(usize, DynTimelineEvalResult)> {
        let target_sec = self.timelines.max_total_secs() * alpha;
        self.eval_sec(target_sec)
    }
    /// Evaluate the timeline at `target_sec`
    pub fn eval_sec(&self, target_sec: f64) -> Option<(usize, DynTimelineEvalResult)> {
        let (timeline_idx, timeline) =
            self.timelines.iter().enumerate().find(|(idx, timeline)| {
                // TODO: make this unwrap better
                let range = timeline.as_timeline().range_sec().unwrap();
                range.contains(&target_sec)
                    || *idx == self.timelines.len() - 1 && range.end == target_sec
            })?;

        match timeline {
            DynTimeline::CameraFrame(inner) => {
                let timeline = (inner.as_ref() as &dyn Any)
                    .downcast_ref::<ItemTimeline<CameraFrame>>()
                    .unwrap();
                timeline
                    .eval_sec(target_sec)
                    .map(|res| (timeline_idx, DynTimelineEvalResult::CameraFrame(res)))
            }
            DynTimeline::VisualItem(inner) => inner
                .eval_sec(target_sec)
                .map(|res| (timeline_idx, DynTimelineEvalResult::VisualItem(res))),
        }
    }
}

impl ItemDynTimelines {
    /// As a reference of [`TimelineFunc`]
    pub fn get_dyn(&self) -> &dyn TimelineFunc {
        // TODO: make this unwrap better
        self.timelines.last().unwrap().as_timeline()
    }
    /// As a mutable reference of [`TimelineFunc`]
    pub fn get_dyn_mut(&mut self) -> &mut dyn TimelineFunc {
        // TODO: make this unwrap better
        self.timelines.last_mut().unwrap().as_timeline_mut()
    }
    /// As a reference of [`ItemTimeline<T>`]
    ///
    /// Should make sure that the last timeline in it is of type `T`
    pub fn get<T: 'static>(&self) -> &ItemTimeline<T> {
        // TODO: make this unwrap better
        self.timelines
            .last()
            .unwrap()
            .as_any()
            .downcast_ref()
            .unwrap()
    }
    /// As a mutable reference of [`ItemTimeline<T>`]
    ///
    /// Should make sure that the last timeline in it is of type `T`
    pub fn get_mut<T: 'static>(&mut self) -> &mut ItemTimeline<T> {
        // TODO: make this unwrap better
        self.timelines
            .last_mut()
            .unwrap()
            .as_any_mut()
            .downcast_mut()
            .unwrap()
    }
    /// Apply a map function.
    ///
    /// This will use the last timeline's state, which is of type `T` to
    /// construct a mapped state of type `E`, then use this mapped state to
    /// create a new [`ItemTimeline<E>`] and push to the end of the timleines.
    ///
    /// If there is a planning static anim, it will get submitted and the new
    /// timeline will start planning a static anim immediately just like the
    /// static anim goes "cross" two timeline of different type.
    pub fn apply_map<T: Clone + 'static, E: Clone + 'static>(&mut self, map_fn: impl FnOnce(T) -> E)
    where
        ItemTimeline<E>: Into<DynTimeline>,
    {
        let (state, end_sec, is_showing) = {
            let timeline = self.get_mut::<T>();
            let is_showing = timeline.planning_static_start_sec.is_some();
            timeline.seal();
            (
                timeline.state().clone(),
                timeline.end_sec().unwrap_or(0.0),
                is_showing,
            )
        };
        let new_state = map_fn(state);
        let mut new_timeline = ItemTimeline::new(new_state);
        new_timeline.forward_to(end_sec);
        if is_showing {
            new_timeline.show();
        }
        self.timelines.push(new_timeline.into());
    }
}

impl TimelineFunc for ItemDynTimelines {
    fn start_sec(&self) -> Option<f64> {
        self.get_dyn().start_sec()
    }
    fn end_sec(&self) -> Option<f64> {
        self.get_dyn().end_sec()
    }
    fn seal(&mut self) {
        self.get_dyn_mut().seal();
    }
    fn cur_sec(&self) -> f64 {
        self.get_dyn().cur_sec()
    }
    fn forward(&mut self, duration_secs: f64) {
        self.get_dyn_mut().forward(duration_secs);
    }
    fn show(&mut self) {
        self.get_dyn_mut().show();
    }
    fn hide(&mut self) {
        self.get_dyn_mut().hide();
    }
    fn get_animation_infos(&self) -> Vec<AnimationInfo> {
        self.timelines
            .iter()
            .flat_map(|timeline| timeline.as_timeline().get_animation_infos())
            .collect()
    }
    fn type_name(&self) -> &str {
        self.get_dyn().type_name()
    }
}

// MARK: ItemTimeline
// ANCHOR: ItemTimeline
/// `ItemTimeline<T>` is used to encode animations for a single type `T`,
/// it contains a list of [`AnimationSpan<T>`] and the corresponding metadata for each span.
pub struct ItemTimeline<T> {
    type_name: String,
    anims: Vec<(AnimationSpan<T>, std::ops::Range<f64>)>,

    // Followings are states use while constructing
    cur_sec: f64,
    /// The state used for static anim.
    state: T,
    /// The start time of the planning static anim.
    /// When it is true, it means that it is showing.
    planning_static_start_sec: Option<f64>,
}
// ANCHOR_END: ItemTimeline

impl<T: 'static> ItemTimeline<T> {
    /// Create a new timeline with the initial state
    ///
    /// The timeline is hidden by default, because we don't know when the first anim starts.
    /// And this allow us to use [`ItemTimeline::forward`] and [`ItemTimeline::forward_to`]
    /// to adjust the start time of the first anim.
    pub(crate) fn new(state: T) -> Self {
        Self {
            type_name: std::any::type_name::<T>().to_string(),
            anims: vec![],
            // extractor: None,
            cur_sec: 0.0,
            state,
            planning_static_start_sec: None,
        }
    }
}

impl<T: Clone + 'static> TimelineFunc for ItemTimeline<T> {
    fn start_sec(&self) -> Option<f64> {
        self.anims.first().map(|(_, range)| range.start)
    }
    fn end_sec(&self) -> Option<f64> {
        self.anims.last().map(|(_, range)| range.end)
    }
    fn seal(&mut self) {
        // println!("seal");
        self._submit_planning_static_anim();
    }
    fn cur_sec(&self) -> f64 {
        self.cur_sec
    }
    /// The [`ItemTimeline::state`] should be `Some`
    fn show(&mut self) {
        self.show();
    }
    fn hide(&mut self) {
        self.hide();
    }
    fn forward(&mut self, duration_secs: f64) {
        self.forward(duration_secs);
    }
    fn get_animation_infos(&self) -> Vec<AnimationInfo> {
        // const MAX_INFO_CNT: usize = 100;
        self.anims
            .iter()
            .map(|(anim, range)| AnimationInfo {
                anim_name: anim.type_name().to_string(),
                range: range.clone(),
            })
            // .take(MAX_INFO_CNT)
            .collect()
    }
    fn type_name(&self) -> &str {
        &self.type_name
    }
}

impl<T: Clone + 'static> ItemTimeline<T> {
    /// Get the current second
    pub fn cur_sec(&self) -> f64 {
        self.cur_sec
    }
    /// Get the current item state
    pub fn state(&self) -> &T {
        &self.state
    }
    /// Update the state
    pub fn update(&mut self, state: T) -> &mut Self {
        self.update_with(|s| *s = state)
    }
    /// Update the state with `update_func`
    pub fn update_with(&mut self, update_func: impl FnOnce(&mut T)) -> &mut Self {
        let showing = self._submit_planning_static_anim();
        update_func(&mut self.state);
        if showing {
            self.show();
        }
        self
    }
    /// Show the item.
    ///
    /// This will start planning an static anim if there isn't an planning static anim.
    pub fn show(&mut self) -> &mut Self {
        if self.planning_static_start_sec.is_none() {
            self.planning_static_start_sec = Some(self.cur_sec)
        }
        self
    }
    /// Hide the item.
    ///
    /// This will submit a static anim if there is an planning static anim.
    pub fn hide(&mut self) -> &mut Self {
        // println!("hide");
        self._submit_planning_static_anim();
        self
    }
    /// Forward the timeline by `secs`
    pub fn forward(&mut self, secs: f64) -> &mut Self {
        self.cur_sec += secs;
        self
    }
    /// Forward the timeline to `target_sec` if the current sec is smaller than it.
    pub fn forward_to(&mut self, target_sec: f64) -> &mut Self {
        if target_sec > self.cur_sec {
            self.forward(target_sec - self.cur_sec);
        }
        self
    }
    fn _submit_planning_static_anim(&mut self) -> bool {
        // println!("{:?}", self.planning_static_start_sec);
        if let Some(start) = self.planning_static_start_sec.take() {
            self.anims.push((
                AnimationSpan::from_evaluator(Evaluator::Static(Arc::new(self.state.clone()))),
                start..self.cur_sec,
            ));
            return true;
        }
        false
    }
    /// Plays an anim with `anim_func`.
    pub fn play_with(&mut self, anim_func: impl FnOnce(T) -> AnimationSpan<T>) -> &mut Self {
        self.play(anim_func(self.state.clone()))
    }
    /// Plays an anim.
    pub fn play(&mut self, anim: AnimationSpan<T>) -> &mut Self {
        self._submit_planning_static_anim();
        let res = anim.eval_alpha(1.0).into_owned();
        let duration = anim.duration_secs;
        let end = self.cur_sec + duration;
        self.anims.push((anim, self.cur_sec..end));
        self.cur_sec = end;
        self.update(res);
        self.show();
        self
    }
    /// Evaluate the state at `alpha`
    pub fn eval_alpha(&self, alpha: f64) -> Option<(EvalResult<T>, usize)> {
        let (Some(start), Some(end)) = (self.start_sec(), self.end_sec()) else {
            return None;
        };
        self.eval_sec(alpha * (end - start) + start)
    }
    /// Evaluate the state at `target_sec`
    pub fn eval_sec(&self, target_sec: f64) -> Option<(EvalResult<T>, usize)> {
        let (Some(start), Some(end)) = (self.start_sec(), self.end_sec()) else {
            return None;
        };

        if !(start..=end).contains(&target_sec) {
            return None;
        }

        self.anims
            .iter()
            .enumerate()
            .find_map(|(idx, (anim, range))| {
                if range.contains(&target_sec)
                    || (idx == self.anims.len() - 1 && target_sec == range.end)
                {
                    Some((idx, anim, range))
                } else {
                    None
                }
            })
            .map(|(idx, anim, range)| {
                let alpha = (target_sec - range.start) / (range.end - range.start);
                (anim.eval_alpha(alpha), idx)
            })
    }
}

// MARK: DynTimeline
// ANCHOR: DynTimeline
/// A type erased [`ItemTimeline<T>`]
///
/// Currently There are two types of Timeline:
/// - [`DynTimeline::CameraFrame`]: Can be created from [`CameraFrame`], has a boxed [`AnyTimelineFunc`] in it.
/// - [`DynTimeline::VisualItem`]: Can be created from [`VisualItem`], has a boxed [`AnyVisualItemTimelineTrait`] in it.
pub enum DynTimeline {
    /// A type erased timeline for [`CameraFrame`], its inner is a boxed [`AnyTimelineFunc`].
    CameraFrame(Box<dyn AnyTimelineFunc>),
    /// A type erased timeline for [`VisualItem`], its inner is a boxed [`AnyVisualItemTimelineTrait`].
    VisualItem(Box<dyn AnyVisualItemTimelineTrait>),
}
// ANCHOR_END: DynTimeline

impl TimelineFunc for DynTimeline {
    fn start_sec(&self) -> Option<f64> {
        self.as_timeline().start_sec()
    }
    fn end_sec(&self) -> Option<f64> {
        self.as_timeline().end_sec()
    }
    fn seal(&mut self) {
        self.as_timeline_mut().seal();
    }
    fn cur_sec(&self) -> f64 {
        self.as_timeline().cur_sec()
    }
    fn forward(&mut self, duration_secs: f64) {
        self.as_timeline_mut().forward(duration_secs);
    }
    fn show(&mut self) {
        self.as_timeline_mut().show();
    }
    fn hide(&mut self) {
        self.as_timeline_mut().hide();
    }
    fn get_animation_infos(&self) -> Vec<AnimationInfo> {
        self.as_timeline().get_animation_infos()
    }
    fn type_name(&self) -> &str {
        self.as_timeline().type_name()
    }
}

impl From<ItemTimeline<CameraFrame>> for DynTimeline {
    fn from(value: ItemTimeline<CameraFrame>) -> Self {
        DynTimeline::CameraFrame(Box::new(value))
    }
}

impl<T: VisualItem + Clone + 'static> From<ItemTimeline<T>> for DynTimeline {
    fn from(value: ItemTimeline<T>) -> Self {
        DynTimeline::VisualItem(Box::new(value))
    }
}

impl DynTimeline {
    /// As a ref of [`TimelineFunc`]
    pub fn as_timeline(&self) -> &dyn TimelineFunc {
        match self {
            DynTimeline::CameraFrame(timeline) => timeline.as_ref() as &dyn TimelineFunc,
            DynTimeline::VisualItem(timeline) => timeline.as_ref() as &dyn TimelineFunc,
        }
    }
    /// As a mut ref of [`TimelineFunc`]
    pub fn as_timeline_mut(&mut self) -> &mut dyn TimelineFunc {
        match self {
            DynTimeline::CameraFrame(timeline) => timeline.as_mut() as &mut dyn TimelineFunc,

            DynTimeline::VisualItem(timeline) => timeline.as_mut() as &mut dyn TimelineFunc,
        }
    }
    /// As a ref of [`Any`]
    pub fn as_any(&self) -> &dyn Any {
        match self {
            DynTimeline::CameraFrame(timeline) => timeline.as_ref() as &dyn Any,
            DynTimeline::VisualItem(timeline) => timeline.as_ref() as &dyn Any,
        }
    }
    /// As a mut ref of [`Any`]
    pub fn as_any_mut(&mut self) -> &mut dyn Any {
        match self {
            DynTimeline::CameraFrame(timeline) => timeline.as_mut() as &mut dyn Any,
            DynTimeline::VisualItem(timeline) => timeline.as_mut() as &mut dyn Any,
        }
    }
}
