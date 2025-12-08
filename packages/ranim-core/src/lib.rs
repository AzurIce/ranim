//! The core of ranim.
//!
//!
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(rustdoc::private_intra_doc_links)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg",
    html_favicon_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg"
)]
#![feature(decl_macro)]
pub mod animation;
/// Color
pub mod color;
/// Component data
pub mod components;
/// The structure to encode animation spans
pub mod timeline;
/// Fondamental traits
pub mod traits;
/// Utils
pub mod utils;

pub mod core_item;
/// The [`core_item::CoreItem`] store
pub mod store;

pub use glam;

/// Prelude
pub mod prelude {
    pub use crate::color::prelude::*;
    pub use crate::traits::*;

    pub use crate::core_item::camera_frame::CameraFrame;
    pub use crate::timeline::{TimelineFunc, TimelinesFunc};
    pub use crate::{RanimScene, TimeMark, TimelineId};
}

use crate::{animation::StaticAnim, core_item::CoreItem, timeline::Timeline};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Extract a [`Extract::Target`] from reference.
pub trait Extract {
    /// The extraction result
    type Target: Clone;
    /// Extract a [`Extract::Target`] from reference.
    fn extract_into(&self, buf: &mut Vec<Self::Target>);
    /// Extract a [`Extract::Target`] from reference.
    fn extract(&self) -> Vec<Self::Target> {
        let mut buf = Vec::new();
        self.extract_into(&mut buf);
        buf
    }
}

impl<E: Extract, I> Extract for I
where
    for<'a> &'a I: IntoIterator<Item = &'a E>,
{
    type Target = E::Target;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        for e in self {
            e.extract_into(buf);
        }
    }
}

use crate::timeline::{AnimationInfo, TimelineFunc, TimelinesFunc};
use tracing::trace;

use std::fmt::Debug;

// MARK: Dylib part
#[doc(hidden)]
#[derive(Clone)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Scene {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub name: &'static str,
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub constructor: fn(&mut RanimScene),
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub config: SceneConfig,
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(skip))]
    pub outputs: &'static [Output],
}

pub use inventory;

inventory::collect!(Scene);

#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn get_scene(idx: usize) -> *const Scene {
    inventory::iter::<Scene>().skip(idx).take(1).next().unwrap()
}

#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn scene_cnt() -> usize {
    inventory::iter::<Scene>().count()
}

/// Return a scene with matched name
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn find_scene(name: &str) -> Option<Scene> {
    inventory::iter::<Scene>().find(|s| s.name == name).cloned()
}

/// Scene config
#[derive(Debug, Clone)]
pub struct SceneConfig {
    /// The height of the frame
    ///
    /// This will be the coordinate in the scene. The width is calculated by the aspect ratio from [`Output::width`] and [`Output::height`].
    pub frame_height: f64,
    /// The clear color
    pub clear_color: &'static str,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            frame_height: 8.0,
            clear_color: "#333333ff",
        }
    }
}

/// The output of a scene
#[derive(Debug, Clone)]
pub struct Output {
    /// The width of the output texture in pixels.
    pub width: u32,
    /// The height of the output texture in pixels.
    pub height: u32,
    /// The frame rate of the output video.
    pub fps: u32,
    /// Whether to save the frames.
    pub save_frames: bool,
    /// The directory to save the output
    ///
    /// Related to the `output` folder, Or absolute.
    pub dir: &'static str,
}

impl Default for Output {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Output {
    /// 1920x1080 60fps save_frames=false dir="./"
    pub const DEFAULT: Self = Self {
        width: 1920,
        height: 1080,
        fps: 60,
        save_frames: false,
        dir: "./",
    };
}

/// TimeMark
#[derive(Debug, Clone)]
pub enum TimeMark {
    /// Capture a picture with a name
    Capture(String),
}

// MARK: SceneConstructor
// ANCHOR: SceneConstructor
/// A scene constructor
///
/// It can be a simple fn pointer of `fn(&mut RanimScene)`,
/// or any type implements `Fn(&mut RanimScene) + Send + Sync`.
pub trait SceneConstructor: Send + Sync {
    /// The construct logic
    fn construct(&self, r: &mut RanimScene);

    /// Use the constructor to build a [`SealedRanimScene`]
    fn build_scene(&self) -> SealedRanimScene {
        let mut scene = RanimScene::new();
        self.construct(&mut scene);
        scene.seal()
    }
}
// ANCHOR_END: SceneConstructor

impl<F: Fn(&mut RanimScene) + Send + Sync> SceneConstructor for F {
    fn construct(&self, r: &mut RanimScene) {
        self(r);
    }
}

/// The id of a timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimelineId(usize);

impl TimelineId {
    /// Get the inner id.
    pub fn id(&self) -> usize {
        self.0
    }
}

// MARK: RanimScene
/// The main struct that offers the ranim's API, and encodes animations
/// The rabjects insert to it will hold a reference to it, so it has interior mutability
#[derive(Default)]
pub struct RanimScene {
    // Timeline<CameraFrame> or Timeline<Item>
    pub(crate) timelines: Vec<Timeline>,
    pub(crate) time_marks: Vec<(f64, TimeMark)>,
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

    /// Create a new timeline.
    pub fn new_timeline(&mut self) -> TimelineId {
        self.new_timeline_with(|_| ())
    }

    /// Create a new timeline and call `f` on it.
    pub fn new_timeline_with(&mut self, f: impl FnOnce(&mut Timeline)) -> TimelineId {
        let id = TimelineId(self.timelines.len());
        let mut timeline = Timeline::new();
        f(&mut timeline);
        self.timelines.push(timeline);
        id
    }

    /// Create a new timeline and play [`StaticAnim::show`] on it at `0.0` sec.
    pub fn insert<T: Extract<Target = CoreItem> + Clone + 'static>(
        &mut self,
        item: T,
    ) -> TimelineId {
        self.insert_at(item, 0.0)
    }

    /// Create a new timeline and play [`StaticAnim::show`] on it at the given sec.
    pub fn insert_at<T: Extract<Target = CoreItem> + Clone + 'static>(
        &mut self,
        item: T,
        sec: f64,
    ) -> TimelineId {
        self.new_timeline_with(|t| {
            t.forward_to(sec);
            t.play(item.show());
        })
    }

    /// Get reference of all timelines
    pub fn timelines(&self) -> &[Timeline] {
        trace!("timelines");
        &self.timelines
    }
    /// Get mutable reference of all timelines
    pub fn timelines_mut(&mut self) -> &mut [Timeline] {
        trace!("timelines_mut");
        &mut self.timelines
    }
    /// Get the reference of timeline(s) by the [`TimelineIndex`].
    pub fn timeline<'a, T: TimelineIndex<'a>>(&'a self, index: T) -> T::RefOutput {
        index.get_index_ref(&self.timelines)
    }
    /// Get the mutable reference of timeline(s) by the [`TimelineIndex`].
    pub fn timeline_mut<'a, T: TimelineIndex<'a>>(&'a mut self, index: T) -> T::MutOutput {
        index.get_index_mut(&mut self.timelines)
    }
    /// Inserts an [`TimeMark`]
    pub fn insert_time_mark(&mut self, sec: f64, time_mark: TimeMark) {
        self.time_marks.push((sec, time_mark));
    }
}

/// The information of an [`Timeline`].
pub struct TimelineInfo {
    /// The inner id value of the [`TimelineId`]
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

// MARK: SealedRanimScene
/// The sealed [`RanimScene`].
///
/// the timelines and time marks cannot be modified after sealed. And
/// once the [`RanimScene`] is sealed, it can be used for evaluating.
pub struct SealedRanimScene {
    pub(crate) total_secs: f64,
    pub(crate) timelines: Vec<Timeline>,
    pub(crate) time_marks: Vec<(f64, TimeMark)>,
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

    /// Get the iterator of timelines
    pub fn timelines_iter(&self) -> impl Iterator<Item = &Timeline> {
        self.timelines.iter()
    }

    /// Get the count of timelines
    pub fn timelines_cnt(&self) -> usize {
        self.timelines.len()
    }

    /// Get timeline infos.
    pub fn get_timeline_infos(&self) -> Vec<TimelineInfo> {
        // const MAX_TIMELINE_CNT: usize = 100;
        self.timelines
            .iter()
            .enumerate()
            // .take(MAX_TIMELINE_CNT)
            .map(|(id, timeline)| TimelineInfo {
                id,
                animation_infos: timeline.get_animation_infos(),
            })
            .collect()
    }

    /// Eval primitives
    pub fn eval_at_sec(&self, target_sec: f64) -> impl Iterator<Item = CoreItem> {
        self.timelines_iter()
            .filter_map(move |t| {
                t.eval_primitives_at_sec(target_sec)
                    .map(|(res, _id_hash)| res)
            })
            .flatten()
    }

    /// Eval primitives
    pub fn eval_at_alpha(&self, alpha: f64) -> impl Iterator<Item = CoreItem> {
        self.eval_at_sec(self.total_secs() * alpha)
    }
}

// MARK: TimelineIndex
/// A trait for indexing timeline(s)
///
/// [`RanimScene::timeline`] and [`RanimScene::timeline_mut`] uses the
/// reference of [`TimelineIndex`] to index the timeline(s).
///
/// See [`TimelineQuery`] for more details.
///
/// | Index Type | Output Type |
/// |------------|-------------|
/// |   `usize`    | `Option<&NeoItemTimeline>` and `Option<&mut NeoItemTimeline>` |
/// |   `TimelineId`    | `&NeoItemTimeline` and `&mut NeoItemTimeline` |
/// |   `TQ: TimelineQuery<'a>`    | `TQ::RessembleResult` and `TQ::RessembleMutResult` |
/// |   `[TQ: TimelineQuery<'a>; N]`    | `[TQ::RessembleResult; N]` and `Result<[TQ::RessembleMutResult; N], TimelineIndexMutError>` |
pub trait TimelineIndex<'a> {
    /// Output of [`TimelineIndex::get_index_ref`]
    type RefOutput;
    /// Output of [`TimelineIndex::get_index_mut`]
    type MutOutput;
    /// Get the reference of timeline(s) from [`RanimScene`] by the [`TimelineIndex`].
    fn get_index_ref(self, timelines: &'a [Timeline]) -> Self::RefOutput;
    /// Get the mutable reference of timeline(s) from [`RanimScene`] by the [`TimelineIndex`].
    fn get_index_mut(self, timelines: &'a mut [Timeline]) -> Self::MutOutput;
}

/// A query of timeline.
///
/// It is implemented for [`TimelineId`], `(TI: AsRef<TimelineId>, T)`, `&(TI: AsRef<TimelineId>, T)` and `&mut (TI: AsRef<TimelineId>, T)`.
///
/// `&(TI: AsRef<TimelineId>, T)` and `&mut (TI: AsRef<TimelineId>, T)` are actually `(TI, &T)` and `(TI, &mut T)`.
pub trait TimelineQuery<'a> {
    /// The result of [`TimelineQuery::ressemble`]
    type RessembleResult;
    /// The result of [`TimelineQuery::ressemble_mut`]
    type RessembleMutResult;
    /// Get the id of the timeline.
    fn id(&self) -> TimelineId;
    /// Ressemble the timeline.
    fn ressemble(self, timeline: &'a Timeline) -> Self::RessembleResult;
    /// Ressemble the mutable timeline.
    fn ressemble_mut(self, timeline: &'a mut Timeline) -> Self::RessembleMutResult;
}

impl<'a> TimelineQuery<'a> for TimelineId {
    type RessembleResult = &'a Timeline;
    type RessembleMutResult = &'a mut Timeline;
    fn id(&self) -> TimelineId {
        *self
    }
    fn ressemble(self, timeline: &'a Timeline) -> Self::RessembleResult {
        timeline
    }
    fn ressemble_mut(self, timeline: &'a mut Timeline) -> Self::RessembleMutResult {
        timeline
    }
}

impl<'a, TI: AsRef<TimelineId>, T> TimelineQuery<'a> for (TI, T) {
    type RessembleResult = (&'a Timeline, T);
    type RessembleMutResult = (&'a mut Timeline, T);
    fn id(&self) -> TimelineId {
        *self.0.as_ref()
    }
    fn ressemble(self, timeline: &'a Timeline) -> Self::RessembleResult {
        (timeline, self.1)
    }
    fn ressemble_mut(self, timeline: &'a mut Timeline) -> Self::RessembleMutResult {
        (timeline, self.1)
    }
}

impl<'a: 'b, 'b, TI: AsRef<TimelineId>, T> TimelineQuery<'a> for &'b (TI, T) {
    type RessembleResult = (&'b Timeline, &'b T);
    type RessembleMutResult = (&'b mut Timeline, &'b T);
    fn id(&self) -> TimelineId {
        *self.0.as_ref()
    }
    fn ressemble(self, timeline: &'a Timeline) -> Self::RessembleResult {
        (timeline, &self.1)
    }
    fn ressemble_mut(self, timeline: &'a mut Timeline) -> Self::RessembleMutResult {
        (timeline, &self.1)
    }
}

impl<'a: 'b, 'b, TI: AsRef<TimelineId>, T> TimelineQuery<'a> for &'b mut (TI, T) {
    type RessembleResult = (&'b Timeline, &'b mut T);
    type RessembleMutResult = (&'b mut Timeline, &'b mut T);
    fn id(&self) -> TimelineId {
        *self.0.as_ref()
    }
    fn ressemble(self, timeline: &'a Timeline) -> Self::RessembleResult {
        (timeline, &mut self.1)
    }
    fn ressemble_mut(self, timeline: &'a mut Timeline) -> Self::RessembleMutResult {
        (timeline, &mut self.1)
    }
}

impl<'a> TimelineIndex<'a> for usize {
    type RefOutput = Option<&'a Timeline>;
    type MutOutput = Option<&'a mut Timeline>;
    fn get_index_ref(self, timelines: &'a [Timeline]) -> Self::RefOutput {
        timelines.get(self)
    }
    fn get_index_mut(self, timelines: &'a mut [Timeline]) -> Self::MutOutput {
        timelines.get_mut(self)
    }
}

impl AsRef<TimelineId> for TimelineId {
    fn as_ref(&self) -> &TimelineId {
        self
    }
}

impl<'a, TQ: TimelineQuery<'a>> TimelineIndex<'a> for TQ {
    type RefOutput = TQ::RessembleResult;
    type MutOutput = TQ::RessembleMutResult;
    fn get_index_ref(self, timelines: &'a [Timeline]) -> Self::RefOutput {
        let id = self.id();
        self.ressemble(id.0.get_index_ref(timelines).unwrap())
    }
    fn get_index_mut(self, timelines: &'a mut [Timeline]) -> Self::MutOutput {
        let id = self.id();
        self.ressemble_mut(id.0.get_index_mut(timelines).unwrap())
    }
}

/// An error of timeline indexing.
#[derive(Debug)]
pub enum TimelineIndexMutError {
    /// The index is overlapping.
    IndexOverlapping,
}

impl<'a, TI: TimelineQuery<'a>, const N: usize> TimelineIndex<'a> for [TI; N] {
    type RefOutput = [TI::RessembleResult; N];
    type MutOutput = Result<[TI::RessembleMutResult; N], TimelineIndexMutError>;
    fn get_index_ref(self, timelines: &'a [Timeline]) -> Self::RefOutput {
        self.map(|x| {
            let id = x.id();
            x.ressemble(id.0.get_index_ref(timelines).unwrap())
        })
    }
    /// Learnt from [`std::slice`]'s `get_disjoint_mut`
    fn get_index_mut(self, timelines: &'a mut [Timeline]) -> Self::MutOutput {
        // Check for overlapping indices
        for (i, idx) in self.iter().enumerate() {
            for idx2 in self[i + 1..].iter() {
                if idx.id() == idx2.id() {
                    return Err(TimelineIndexMutError::IndexOverlapping);
                }
            }
        }

        // Collect all indices first
        let indices: [usize; N] = std::array::from_fn(|i| self[i].id().0);

        // NB: This implementation is written as it is because any variation of
        // `indices.map(|i| self.get_unchecked_mut(i))` would make miri unhappy,
        // or generate worse code otherwise. This is also why we need to go
        // through a raw pointer here.
        let mut arr: std::mem::MaybeUninit<[TI::RessembleMutResult; N]> =
            std::mem::MaybeUninit::uninit();
        let arr_ptr = arr.as_mut_ptr();
        let timelines_ptr: *mut Timeline = timelines.as_mut_ptr();
        let self_manually_drop = std::mem::ManuallyDrop::new(self);

        // SAFETY: We've verified that all indices are disjoint and in bounds.
        // We use raw pointers to get multiple mutable references to different
        // elements of the slice, which is safe because the indices are disjoint.
        // We use ManuallyDrop to prevent double-drop of self's elements after
        // reading them with ptr::read.
        let res = unsafe {
            for (i, &idx) in indices.iter().enumerate() {
                let timeline_ref = &mut *timelines_ptr.add(idx);
                let ti = std::ptr::read(self_manually_drop.as_ptr().add(i));
                arr_ptr
                    .cast::<TI::RessembleMutResult>()
                    .add(i)
                    .write(ti.ressemble_mut(timeline_ref));
            }
            arr.assume_init()
        };

        Ok(res)
    }
}
