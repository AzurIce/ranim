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
/// Scene item storage.
pub mod store;
/// Fundamental traits.
pub mod traits;
/// Utilities.
pub mod utils;

pub use glam;
pub use num;

use std::fmt::Debug;

use animation::{AnimSequence, Animation, BuiltAnimation};
use core_item::CoreItem;

/// Commonly used ranim APIs.
pub mod prelude {
    pub use crate::color::prelude::*;
    pub use crate::traits::*;

    pub use crate::animation::{AnimSequence, AnimStack, Animation, Placeable, StaticAnim};
    pub use crate::core_item::camera_frame::CameraFrame;
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
/// The built-in [`AnimSequence`] is also available independently for users to
/// construct reusable dynamic animation groups. Calling [`RanimScene::play`]
/// flattens a statically typed animation into this sequence and performs the
/// single evaluator type-erasure step.
#[derive(Default)]
pub struct RanimScene {
    anims: AnimSequence,
    time_marks: Vec<(f64, TimeMark)>,
}

impl RanimScene {
    /// Create an empty scene definition.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an animation at the scene's current cursor.
    pub fn play<A: Animation>(&mut self, animation: A) -> &mut Self {
        self.anims.play(animation);
        self
    }

    /// Append a user-built animation sequence at the current cursor.
    pub fn extend(&mut self, sequence: AnimSequence) -> &mut Self {
        self.anims.extend(sequence);
        self
    }

    /// Advance the scene cursor without adding an animation.
    pub fn forward(&mut self, secs: f64) -> &mut Self {
        self.anims.forward(secs);
        self
    }

    /// Advance the scene cursor to `target_sec` without adding an animation.
    pub fn forward_to(&mut self, target_sec: f64) -> &mut Self {
        self.anims.forward_to(target_sec);
        self
    }

    /// Advance the scene cursor while holding its current evaluated state.
    pub fn hold(&mut self, secs: f64) -> &mut Self {
        self.anims.hold(secs);
        self
    }

    /// Advance the scene cursor to `target_sec` while holding its current state.
    pub fn hold_to(&mut self, target_sec: f64) -> &mut Self {
        self.anims.hold_to(target_sec);
        self
    }

    /// Borrow the root animation sequence.
    pub fn animations(&self) -> &AnimSequence {
        &self.anims
    }

    /// Mutably borrow the root animation sequence.
    pub fn animations_mut(&mut self) -> &mut AnimSequence {
        &mut self.anims
    }

    /// Insert a time mark.
    pub fn insert_time_mark(&mut self, sec: f64, time_mark: TimeMark) {
        self.time_marks.push((sec, time_mark));
    }

    /// Finish the definition and produce an immutable, evaluable recipe.
    pub fn seal(self) -> SealedRanimScene {
        let total_secs = self.anims.cursor_sec();
        SealedRanimScene {
            total_secs,
            animations: self.anims.into_built_animations(),
            time_marks: self.time_marks,
        }
    }
}

impl Debug for RanimScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RanimScene")
            .field("animations", &self.anims.built_animations().len())
            .field("duration_secs", &self.anims.cursor_sec())
            .finish()
    }
}

/// Lightweight animation information used by preview tooling.
pub struct AnimationInfo {
    /// Concrete evaluator name.
    pub anim_name: String,
    /// Global time range.
    pub range: std::ops::Range<f64>,
}

/// One preview row of animation information.
///
/// The new model has a single root animation container; nested composition can
/// later provide richer editor grouping without restoring the old Timeline API.
pub struct TimelineInfo {
    /// Preview row identifier.
    pub id: usize,
    /// Flattened animations shown in this row.
    pub animation_infos: Vec<AnimationInfo>,
}

/// Immutable animation recipe produced by [`RanimScene::seal`].
pub struct SealedRanimScene {
    total_secs: f64,
    animations: Vec<BuiltAnimation>,
    time_marks: Vec<(f64, TimeMark)>,
}

impl SealedRanimScene {
    /// Total scene duration.
    pub fn total_secs(&self) -> f64 {
        self.total_secs
    }

    /// Scene time marks.
    pub fn time_marks(&self) -> &[(f64, TimeMark)] {
        &self.time_marks
    }

    /// Flattened animation information for the current preview UI.
    pub fn get_timeline_infos(&self) -> Vec<TimelineInfo> {
        vec![TimelineInfo {
            id: 0,
            animation_infos: self
                .animations
                .iter()
                .map(|animation| AnimationInfo {
                    anim_name: animation.anim_name().to_string(),
                    range: animation.time_range(),
                })
                .collect(),
        }]
    }

    /// Evaluate all clips active at `target_sec` and extract scene primitives.
    pub fn eval_at_sec(&self, target_sec: f64) -> impl Iterator<Item = ((usize, usize), CoreItem)> {
        self.animations
            .iter()
            .enumerate()
            .filter_map(move |(animation_id, animation)| {
                if !animation.enabled() {
                    return None;
                }

                let range = animation.time_range();
                let active = range.contains(&target_sec)
                    || (target_sec == self.total_secs && target_sec == range.end);
                active
                    .then(|| animation.eval_at_sec(target_sec))
                    .flatten()
                    .map(move |items| (animation_id, items))
            })
            .flat_map(|(animation_id, items)| {
                items
                    .into_iter()
                    .flat_map(|item| item.extract())
                    .map(move |item| ((0, animation_id), item))
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
        animation::{Eval, Placeable, Static},
        core_item::vitem::VItem,
    };

    fn leaf(duration: f64) -> impl Animation + Placeable {
        Static(VItem::default())
            .into_animation_cell()
            .with_duration(duration)
    }

    #[test]
    fn scene_play_builds_into_the_root_sequence() {
        let mut scene = RanimScene::new();
        scene.play(chain![leaf(2.0), leaf(3.0)]);
        let sealed = scene.seal();

        assert_eq!(sealed.total_secs(), 5.0);
        let infos = sealed.get_timeline_infos();
        assert_eq!(infos[0].animation_infos[0].range, 0.0..2.0);
        assert_eq!(infos[0].animation_infos[1].range, 2.0..5.0);
    }

    #[test]
    fn user_sequence_can_be_extended_at_the_scene_cursor() {
        let mut reusable = AnimSequence::new();
        reusable.play(leaf(2.0)).forward(1.0).play(leaf(1.0));

        let mut scene = RanimScene::new();
        scene.forward(5.0).extend(reusable);
        let sealed = scene.seal();

        assert_eq!(sealed.total_secs(), 9.0);
        let infos = sealed.get_timeline_infos();
        assert_eq!(infos[0].animation_infos[0].range, 5.0..7.0);
        assert_eq!(infos[0].animation_infos[1].range, 8.0..9.0);
    }
}
