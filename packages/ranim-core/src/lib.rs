//! The core of ranim.

#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(rustdoc::private_intra_doc_links)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg",
    html_favicon_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg"
)]

/// Anchors and semantic bounds.
pub mod anchor;
pub mod animation;
/// Color utilities.
pub mod color;
/// Component data.
pub mod components;
/// Fundamental scene primitives.
pub mod core_item;
/// Scene evaluation driver (lightweight session, no ECS).
pub mod scene_evaluator;
/// Time vocabulary for animation evaluation.
pub mod time;
/// Fundamental traits.
pub mod traits;
pub use scene_evaluator::SceneEvaluator;
/// Utilities.
pub mod utils;

pub use glam;
pub use num;
use std::fmt::Debug;

use animation::{AnimStack, Animation, AnimationCell};
pub use animation::{AnimationInfo, AnimationInfoKind};
use core_item::CoreItem;

/// Commonly used ranim APIs.
pub mod prelude {
    pub use crate::color::prelude::*;
    pub use crate::traits::*;

    pub use crate::animation::{
        AnimIterExt, AnimLagged, AnimSequence, AnimStack, Animation, AnimationExt, Eval,
        LaggedFill, Placeable, StaticAnim,
    };
    pub use crate::core_item::camera_frame::CameraFrame;
    pub use crate::time::{DeltaTime, Time};
    pub use crate::{RanimScene, TimeMark};
}

/// Extract one or more target values from a reference.
pub trait Extract {
    /// Extraction target.
    type Target: Clone;
    /// Append extracted values to `buf`.
    fn extract_into(&self, buf: &mut Vec<Self::Target>);
    /// Extract into a newly allocated vector.
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
        for element in self {
            element.extract_into(buf);
        }
    }
}

/// A marker attached to a time in a scene definition.
#[derive(Debug, Clone)]
pub enum TimeMark {
    /// Capture a picture with a name.
    Capture(String),
}

/// Animation definition builder passed to scene constructors.
///
/// The public [`RanimScene::root`] stack is the scene's animation composition
/// root. Calling [`RanimScene::play`] is a convenience alias for pushing into
/// that stack.
#[derive(Default)]
pub struct RanimScene {
    /// Root animation stack. Modules pushed here share the same local origin.
    pub root: AnimStack,
    time_marks: Vec<(f64, TimeMark)>,
}

impl RanimScene {
    /// Create an empty scene definition.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push an animation module into the root stack.
    pub fn play<A: Animation + 'static>(&mut self, animation: A) -> &mut Self {
        self.root.push(animation);
        self
    }

    /// Insert a time mark.
    pub fn insert_time_mark(&mut self, sec: f64, time_mark: TimeMark) {
        self.time_marks.push((sec, time_mark));
    }

    /// Finish the definition and produce an immutable, evaluable recipe.
    pub fn seal(self) -> SealedRanimScene {
        let total_secs = self.root.duration_secs();
        SealedRanimScene {
            total_secs,
            animations: self.root.into_built_animations(),
            time_marks: self.time_marks,
        }
    }
}

impl Debug for RanimScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RanimScene")
            .field("animations", &self.root.built_animations().len())
            .field("duration_secs", &self.root.duration_secs())
            .finish()
    }
}

/// Immutable animation recipe produced by [`RanimScene::seal`].
pub struct SealedRanimScene {
    total_secs: f64,
    animations: Vec<AnimationCell>,
    time_marks: Vec<(f64, TimeMark)>,
}

impl SealedRanimScene {
    /// Consume this scene into a [`SceneEvaluator`] driving session.
    ///
    /// `logic_fps` is the fixed logic grid resolution; the time model defaults
    /// to 120 Hz. Iterative (stateful) segments require the evaluator: the pure
    /// [`eval_at_sec`](Self::eval_at_sec) path does not advance their state.
    pub fn into_evaluator(self, logic_fps: f64) -> SceneEvaluator {
        SceneEvaluator::new(self, logic_fps)
    }

    /// Total scene duration.
    pub fn total_secs(&self) -> f64 {
        self.total_secs
    }

    /// Scene time marks.
    pub fn time_marks(&self) -> &[(f64, TimeMark)] {
        &self.time_marks
    }

    /// Hierarchical runtime animation information for preview tooling.
    pub fn get_animation_infos(&self) -> Vec<AnimationInfo> {
        self.animations
            .iter()
            .map(AnimationCell::animation_info)
            .collect()
    }

    /// Sample all clips active at `target_sec` and extract scene primitives.
    ///
    /// This is the pure query path: it samples the **current** state of every
    /// segment without advancing anything. Stateful (iterative) segments
    /// project whatever state they currently hold — drive them with a
    /// [`SceneEvaluator`] session first for meaningful results.
    pub fn eval_at_sec(&self, target_sec: f64) -> impl Iterator<Item = ((usize, usize), CoreItem)> {
        self.animations
            .iter()
            .enumerate()
            .filter_map(move |(animation_id, animation)| {
                if !animation.enabled() {
                    return None;
                }

                let mut items = Vec::new();
                animation.sample_at_sec(target_sec, target_sec, &mut items);
                (!items.is_empty()).then_some((animation_id, items))
            })
            .flat_map(|(animation_id, items)| {
                items
                    .into_iter()
                    .flat_map(|item| item.extract())
                    .enumerate()
                    .map(move |(part, item)| ((animation_id, part), item))
            })
    }

    /// Evaluate by normalized scene progress.
    pub fn eval_at_alpha(&self, alpha: f64) -> impl Iterator<Item = ((usize, usize), CoreItem)> {
        self.eval_at_sec(self.total_secs * alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        animation::{AnimSequence, AnimationExt, Placeable, Static},
        core_item::vitem::VItem,
    };

    fn leaf(duration: f64) -> impl Placeable {
        Static(VItem::default()).with_duration(duration)
    }

    #[test]
    fn scene_play_pushes_into_the_root_stack() {
        let mut scene = RanimScene::new();
        scene.play(seq![leaf(2.0), leaf(3.0)]);
        let sealed = scene.seal();

        assert_eq!(sealed.total_secs(), 5.0);
        let infos = sealed.get_animation_infos();
        assert_eq!(infos[0].range, 0.0..5.0);
        assert_eq!(infos[0].children[0].range, 0.0..2.0);
        assert_eq!(infos[0].children[1].range, 2.0..5.0);
    }

    #[test]
    fn extracted_items_have_unique_semantic_part_ids() {
        let mut scene = RanimScene::new();
        scene.play(Static(vec![VItem::default(), VItem::default()]).with_duration(1.0));
        let sealed = scene.seal();

        let ids = sealed
            .eval_at_sec(0.5)
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        assert_eq!(ids, [(0, 0), (0, 1)]);
    }

    #[test]
    fn scene_modules_share_the_root_origin() {
        let mut reusable = AnimSequence::new();
        reusable.push(leaf(2.0)).forward(1.0).push(leaf(1.0));

        let mut scene = RanimScene::new();
        scene.play(reusable);
        scene.root.push(leaf(5.0));
        let sealed = scene.seal();

        assert_eq!(sealed.total_secs(), 5.0);
        let infos = sealed.get_animation_infos();
        assert_eq!(infos[0].range, 0.0..4.0);
        assert_eq!(infos[0].children[0].range, 0.0..2.0);
        assert_eq!(infos[0].children[1].range, 3.0..4.0);
        assert_eq!(infos[1].range, 0.0..5.0);
    }
}
