use itertools::Itertools;
use log::{trace, warn};
use ranim_core::Extract;
use ranim_core::primitives::Primitives;
use ranim_core::timeline::{
    AnimationInfo, ItemDynTimelines, ItemTimeline, TimelineFunc, TimelinesFunc,
};

use crate::items::ItemId;
use crate::render::primitives::Renderable;
use ranim_core::primitives::camera_frame::CameraFrame;
use std::{any::TypeId, fmt::Debug};

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
    /// (`EvalResult<CameraFrame>`, `animation idx`)
    pub camera_frame: (CameraFrame, usize),
    /// (`id`, `EvalResult<Box<dyn RenderableItem>>`, `animation idx`)
    pub visual_items: Vec<(usize, Box<dyn Renderable>, usize)>,
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
    pub fn insert<T: Extract + Clone + 'static>(&mut self, state: T) -> ItemId<T> {
        self.insert_and(state, |_| {})
    }
    /// Use the item state to create a new [`ItemDynTimelines`], and call [`ItemTimeline::show`] on it.
    pub fn insert_and_show<T: Extract + Clone + 'static>(&mut self, state: T) -> ItemId<T> {
        self.insert_and(state, |timeline| {
            timeline.show();
        })
    }
    /// Use the item state to create a new [`ItemDynTimelines`], and call `f` on it.
    pub fn insert_and<T: Extract + Clone + 'static>(
        &mut self,
        state: T,
        f: impl FnOnce(&mut ItemTimeline<T>),
    ) -> ItemId<T> {
        let id = ItemId::new(self.timelines.len());
        trace!("insert_and type of {:?}, id: {id:?}", TypeId::of::<T>());
        let mut item_timeline = ItemTimeline::<T>::new(state);
        f(&mut item_timeline);

        let mut timeline = ItemDynTimelines::new();
        timeline.push(item_timeline);
        self.timelines.push(timeline);
        id
    }
    /// Consumes an [`ItemId<T>`], and convert it into [`ItemId<E>`].
    ///
    /// This insert inserts an [`ItemTimeline<E>`] into the corresponding [`ItemDynTimelines`]
    pub fn map<T: Extract + Clone + 'static, E: Extract + Clone + 'static>(
        &mut self,
        item_id: ItemId<T>,
        map_fn: impl FnOnce(T) -> E,
    ) -> ItemId<E> {
        trace!(
            "map {item_id:?} {:?} -> {:?}",
            TypeId::of::<T>(),
            TypeId::of::<E>()
        );
        // let dyn_item_timeline = self
        //     .timelines
        //     .iter_mut()
        //     .find(|timeline| timeline.id == *item_id)
        //     .unwrap();
        let dyn_item_timeline = self.timelines.get_mut(*item_id).unwrap();
        dyn_item_timeline.apply_map(map_fn);
        ItemId::new(item_id.inner())
    }

    /// Get reference of all timelines in the type erased [`ItemDynTimelines`] type.
    pub fn timelines(&self) -> &[ItemDynTimelines] {
        trace!("timelines");
        &self.timelines
    }
    /// Get mutable reference of all timelines in the type erased [`ItemDynTimelines`] type.
    pub fn timelines_mut(&mut self) -> &mut [ItemDynTimelines] {
        trace!("timelines_mut");
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
    pub fn time_marks(&self) -> &[(f64, TimeMark)] {
        &self.time_marks
    }
    /// Evaluate the state of timelines at `target_sec`
    pub fn eval_sec(&self, target_sec: f64) -> TimelineEvalResult {
        let mut items = Vec::with_capacity(self.timelines.len());

        let mut camera_frame = None::<(CameraFrame, usize)>;

        for (id, timeline) in self.timelines.iter().enumerate() {
            // println!("### eval_sec timeline id {id} ###");
            let Some((res, anim_idx)) = timeline.eval_sec_extracted_any(target_sec) else {
                continue;
            };
            // ! Note that the `TypeId` between different compile units maybe different.
            // if let Some(x) = res.downcast_ref::<CameraFrame>() {
            //     println!("Camera frame found at sec {target_sec} with anim idx {id}");
            //     camera_frame = Some((x.clone(), anim_idx));
            // } else if let Some(x) = res.downcast_ref::<VItemPrimitive>() {
            //     println!("Visual item found at sec {target_sec} with anim idx {id}");
            //     items.push((id, x.clone(), anim_idx));
            // }
            match res.into_owned() {
                Primitives::CameraFrame(res) => {
                    // println!("CameraFrame found at sec timeline id {id}");
                    camera_frame = Some((res[0].clone(), anim_idx));
                }
                Primitives::VItemPrimitive(res) => {
                    // println!("VItemPrimitive found at sec timeline id {id}");
                    items.push((id, Box::new(res) as Box<dyn Renderable>, anim_idx));
                }
            }

            // match res {
            //     DynTimelineEvalResult::CameraFrame((res, idx)) => {
            //         camera_frame = Some((res, timeline_idx, idx))
            //     }
            //     DynTimelineEvalResult::VisualItem((res, idx)) => {
            //         items.push((timeline.id, res, timeline_idx, idx));
            //     }
            // }
        }

        if camera_frame.is_none() {
            warn!("No camera frame found at sec {target_sec}");
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
            .enumerate()
            // .take(MAX_TIMELINE_CNT)
            .map(|(id, timeline)| ItemDynTimelinesInfo {
                id,
                animation_infos: timeline.get_animation_infos(),
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
    fn timeline(&self, r: &'a RanimScene) -> Self::RefOutput {
        r.timelines.get(*self)
        // timeline
        //     .timelines()
        //     .iter()
        //     .find(|timeline| *self == timeline.id)
    }
    fn timeline_mut(&self, r: &'a mut RanimScene) -> Self::MutOutput {
        r.timelines.get_mut(*self)
        // timeline
        //     .timelines_mut()
        //     .iter_mut()
        //     .find(|timeline| *self == timeline.id)
    }
}

impl<'a, T: 'static> TimelineIndex<'a> for ItemId<T> {
    type RefOutput = &'a ItemTimeline<T>;
    type MutOutput = &'a mut ItemTimeline<T>;
    fn timeline(&self, r: &'a RanimScene) -> Self::RefOutput {
        r.timelines.get(**self).unwrap().get()
        // timeline
        //     .timelines()
        //     .iter()
        //     .find(|timeline| **self == timeline.id)
        //     .map(|timeline| timeline.get())
        //     .unwrap()
    }
    fn timeline_mut(&self, r: &'a mut RanimScene) -> Self::MutOutput {
        r.timelines.get_mut(**self).unwrap().get_mut()
        // r
        //     .timelines_mut()
        //     .iter_mut()
        //     .find(|timeline| **self == timeline.id)
        //     .map(|timeline| timeline.get_mut())
        //     .unwrap()
    }
}

impl<'a, T: 'static, const N: usize> TimelineIndex<'a> for [&ItemId<T>; N] {
    type RefOutput = [&'a ItemTimeline<T>; N];
    type MutOutput = [&'a mut ItemTimeline<T>; N];
    fn timeline(&self, r: &'a RanimScene) -> Self::RefOutput {
        // TODO: the order is not stable
        let mut timelines = r
            .timelines()
            .iter()
            .enumerate()
            .filter(|(id, _)| self.iter().any(|target_id| ***target_id == *id))
            .collect_array::<N>()
            .unwrap();
        timelines.sort_by_key(|(id, _)| {
            self.iter()
                .position(|target_id| ***target_id == *id)
                .unwrap()
        });
        timelines.map(|(_, timeline)| timeline.get())
    }
    fn timeline_mut(&self, r: &'a mut RanimScene) -> Self::MutOutput {
        // TODO: the order is not stable
        let mut timelines = r
            .timelines_mut()
            .iter_mut()
            .enumerate()
            .filter(|(id, _)| self.iter().any(|target_id| ***target_id == *id))
            .collect_array::<N>()
            .unwrap();
        timelines.sort_by_key(|(id, _)| {
            self.iter()
                .position(|target_id| ***target_id == *id)
                .unwrap()
        });
        timelines.map(|(_, timeline)| timeline.get_mut())
    }
}
