use itertools::Itertools;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    animation::{AnimationSpan, EvalResult, Evaluator},
    items::{ItemId, VisualItem, camera_frame::CameraFrame},
};
use std::fmt::Debug;
use std::{any::Any, sync::Arc};

#[derive(Debug, Clone)]
pub enum TimeMark {
    Capture(String),
}

#[allow(clippy::type_complexity)]
pub struct TimelineEvalResult {
    pub camera_frame: (EvalResult<CameraFrame>, usize, usize),
    /// (`id`, `EvalResult<Box<dyn RenderableItem>>`, `timeline idx` `animation idx`)
    pub visual_items: Vec<(usize, EvalResult<Box<dyn VisualItem>>, usize, usize)>,
}

/// Timeline is a type erased [`ItemTimeline<T>`]
///
/// Currently There are two types of Timeline:
/// - [`DynTimeline::VisualItem`]: Can be created from [`VisualItem`], has a boxed [`AnyVisualItemTimelineTrait`] in it.
/// - [`DynTimeline::CameraFrame`]: Can be created from [`CameraFrame`], has a boxed [`AnyTimelineTrait`] in it.
pub enum DynTimeline {
    CameraFrame(Box<dyn AnyTimelineTrait>),
    VisualItem(Box<dyn AnyVisualItemTimelineTrait>),
}

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

    fn elapsed_secs(&self) -> f64 {
        self.as_timeline().elapsed_secs()
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
    pub fn as_timeline(&self) -> &dyn TimelineFunc {
        match self {
            DynTimeline::CameraFrame(timeline) => timeline.as_timeline(),
            DynTimeline::VisualItem(timeline) => timeline.as_timeline(),
        }
    }
    pub fn as_timeline_mut(&mut self) -> &mut dyn TimelineFunc {
        match self {
            DynTimeline::CameraFrame(timeline) => timeline.as_timeline_mut(),
            DynTimeline::VisualItem(timeline) => timeline.as_timeline_mut(),
        }
    }
    pub fn as_any(&self) -> &dyn Any {
        match self {
            DynTimeline::CameraFrame(timeline) => timeline.as_ref() as &dyn Any,
            DynTimeline::VisualItem(timeline) => timeline.as_ref() as &dyn Any,
        }
    }
    pub fn as_any_mut(&mut self) -> &mut dyn Any {
        match self {
            DynTimeline::CameraFrame(timeline) => timeline.as_mut() as &mut dyn Any,
            DynTimeline::VisualItem(timeline) => timeline.as_mut() as &mut dyn Any,
        }
    }
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

    pub fn insert<T: Clone + 'static>(&mut self, state: T) -> ItemId<T>
    where
        ItemTimeline<T>: Into<DynTimeline>,
    {
        self.insert_and(state, |_| {})
    }
    pub fn insert_and_show<T: Clone + 'static>(&mut self, state: T) -> ItemId<T>
    where
        ItemTimeline<T>: Into<DynTimeline>,
    {
        self.insert_and(state, |timeline| {
            timeline.show();
        })
    }
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

    pub fn timelines(&self) -> &Vec<ItemDynTimelines> {
        &self.timelines
    }
    pub fn timelines_mut(&mut self) -> &mut Vec<ItemDynTimelines> {
        &mut self.timelines
    }
    pub fn timeline<'a, T: TimelineIndex<'a>>(&'a self, index: &T) -> T::RefOutput {
        index.timeline(self)
    }
    pub fn timeline_mut<'a, T: TimelineIndex<'a>>(&'a mut self, index: &T) -> T::MutOutput {
        index.timeline_mut(self)
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
    timelines: Vec<ItemDynTimelines>,
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
    fn timeline(&self, timeline: &'a RanimScene) -> Self::RefOutput;
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
    pub range: std::ops::Range<f64>,
}

// MARK: TimelineFunc
pub trait TimelineFunc {
    fn start_sec(&self) -> Option<f64>;
    fn end_sec(&self) -> Option<f64>;
    fn range_sec(&self) -> Option<std::ops::Range<f64>> {
        let (Some(start), Some(end)) = (self.start_sec(), self.end_sec()) else {
            return None;
        };
        Some(start..end)
    }
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
    // fn show_secs(&self) -> &Vec<f64>;
}

pub struct ItemDynTimelines {
    id: usize,
    timelines: Vec<DynTimeline>,
}

pub enum DynTimelineEvalResult {
    CameraFrame((EvalResult<CameraFrame>, usize)),
    VisualItem((EvalResult<Box<dyn VisualItem>>, usize)),
}

impl ItemDynTimelines {
    pub fn eval_alpha(&self, alpha: f64) -> Option<(usize, DynTimelineEvalResult)> {
        let target_sec = self.timelines.max_total_secs() * alpha;
        self.eval_sec(target_sec)
    }
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
    pub fn get_dyn(&self) -> &dyn TimelineFunc {
        // TODO: make this unwrap better
        self.timelines.last().unwrap().as_timeline()
    }
    pub fn get_dyn_mut(&mut self) -> &mut dyn TimelineFunc {
        // TODO: make this unwrap better
        self.timelines.last_mut().unwrap().as_timeline_mut()
    }
    pub fn get<T: 'static>(&self) -> &ItemTimeline<T> {
        // TODO: make this unwrap better
        self.timelines
            .last()
            .unwrap()
            .as_any()
            .downcast_ref()
            .unwrap()
    }
    pub fn get_mut<T: 'static>(&mut self) -> &mut ItemTimeline<T> {
        // TODO: make this unwrap better
        self.timelines
            .last_mut()
            .unwrap()
            .as_any_mut()
            .downcast_mut()
            .unwrap()
    }
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
                timeline.elapsed_secs(),
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

    fn elapsed_secs(&self) -> f64 {
        self.get_dyn().elapsed_secs()
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
/// `ItemTimeline<T>` is used to encode animations for a single type `T`,
/// it contains a list of [`AnimationSpan<T>`] and the corresponding metadata for each span.
pub struct ItemTimeline<T> {
    type_name: String,
    anims: Vec<(AnimationSpan<T>, std::ops::Range<f64>)>,
    // extractor: Option<E>,

    // State for building the clip
    cur_sec: f64,
    /// The state used for static anim.
    state: T,
    /// The start time of the planning static anim.
    /// When it is true, it means that it is showing.
    planning_static_start_sec: Option<f64>,
}

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
    fn elapsed_secs(&self) -> f64 {
        self.end_sec().unwrap_or(0.0)
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
    pub fn cur_sec(&self) -> f64 {
        self.cur_sec
    }
    pub fn state(&self) -> &T {
        &self.state
    }
    pub fn update_with(&mut self, update_func: impl FnOnce(&mut T)) -> &mut Self {
        let showing = self._submit_planning_static_anim();
        update_func(&mut self.state);
        if showing {
            self.show();
        }
        self
    }
    pub fn update(&mut self, state: T) -> &mut Self {
        let showing = self._submit_planning_static_anim();
        self.state = state;
        if showing {
            self.show();
        }
        self
    }
    /// The [`ItemTimeline::state`] should be `Some`
    pub fn show(&mut self) -> &mut Self {
        if self.planning_static_start_sec.is_none() {
            self.planning_static_start_sec = Some(self.cur_sec)
        }
        self
    }
    pub fn hide(&mut self) -> &mut Self {
        // println!("hide");
        self._submit_planning_static_anim();
        self
    }
    pub fn forward(&mut self, duration_secs: f64) -> &mut Self {
        self.cur_sec += duration_secs;
        self
    }
    pub fn forward_to(&mut self, target_sec: f64) -> &mut Self {
        if target_sec > self.cur_sec {
            self.forward(target_sec - self.cur_sec);
        }
        self
    }
    fn push_anim(&mut self, anim: AnimationSpan<T>, start: f64, end: f64) {
        self.anims.push((anim, start..end));
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
    pub fn play_with(&mut self, anim_func: impl FnOnce(T) -> AnimationSpan<T>) -> &mut Self {
        self.play(anim_func(self.state.clone()))
    }
    pub fn play(&mut self, anim: AnimationSpan<T>) -> &mut Self {
        self._submit_planning_static_anim();
        let res = anim.eval_alpha(1.0).into_owned();
        let duration = anim.duration_secs;
        let end = self.cur_sec + duration;
        self.push_anim(anim, self.cur_sec, end);
        self.cur_sec = end;
        self.update(res);
        self.show();
        self
    }
    pub fn eval_alpha(&self, alpha: f64) -> Option<(EvalResult<T>, usize)> {
        let (Some(start), Some(end)) = (self.start_sec(), self.end_sec()) else {
            return None;
        };
        self.eval_sec(alpha * (end - start) + start)
    }
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
