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
/// The audio plane of a scene.
pub mod audio;
/// Color utilities.
pub mod color;
/// Component data.
pub mod components;
/// Reusable typed output of scene evaluation.
pub mod frame;
/// Scene evaluation driver (lightweight session, no ECS).
pub mod scene_evaluator;
/// Time vocabulary for animation evaluation.
pub mod time;
/// Fundamental traits.
pub mod traits;
/// Generic values paired with typed external transforms.
pub mod transformed;
pub use scene_evaluator::SceneEvaluator;
/// Utilities.
pub mod utils;

pub use glam;
use std::fmt::Debug;
use std::sync::Arc;

use animation::{
    build::IntoAnimNode,
    compose::stack::AnimStack,
    eval::OutputContext,
    node::{AnimNode, AnimationInfo, bake_audio},
};
use frame::EvaluatedFrame;

/// Commonly used ranim APIs.
pub mod prelude {
    pub use crate::color::prelude::*;
    pub use crate::traits::*;

    pub use crate::animation::build::{IntoAnimNode, PlaybackExt, Unplaced};
    pub use crate::animation::compose::{
        AnimIterExt,
        lagged::{AnimLagged, LaggedFill},
        sequence::AnimSequence,
        stack::AnimStack,
    };
    pub use crate::animation::eval::{
        Eval, EvalExt, Static, StaticAnim,
        iterative::{Iterative, IterativeEval, IterativeFn},
        pure::Pure,
    };
    pub use crate::animation::sound::Sound;
    pub use crate::audio::{AudioClip, AudioTrack};
    pub use crate::transformed::{Transformed, TransformedExt};
    pub use crate::{RanimScene, TimeMark};
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
    ///
    /// Audio leaves ([`Sound`](crate::animation::sound::Sound)) compose in the same tree beside visual
    /// animations; [`RanimScene::seal`] bakes them through the same cell
    /// remaps the visuals experience.
    pub root: AnimStack,
    time_marks: Vec<(f64, TimeMark)>,
}

impl RanimScene {
    /// Create an empty scene definition.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push an animation module into the root stack.
    pub fn play<A: IntoAnimNode + 'static>(&mut self, animation: A) -> &mut Self {
        self.root.push(animation);
        self
    }

    /// Insert a time mark.
    pub fn insert_time_mark(&mut self, sec: f64, time_mark: TimeMark) {
        self.time_marks.push((sec, time_mark));
    }

    /// Finish the definition and produce an immutable, evaluable recipe.
    ///
    /// The tree is the only animation representation: visual cells are
    /// point-walked per frame by [`SceneEvaluator::sample_at`], and sound
    /// leaves are baked here — see [`bake_audio`](crate::animation) — both
    /// experiencing the same cell remaps along the path.
    pub fn seal(self) -> SealedRanimScene {
        let total_secs = self.root.duration_secs();
        let animations = self.root.into_built_animations();
        let sample_rate = crate::audio::MASTER_SAMPLE_RATE as f64;
        let audio: Arc<[f32]> = bake_audio(&animations, total_secs, sample_rate).into();
        SealedRanimScene {
            total_secs,
            animations,
            audio,
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
    animations: Vec<AnimNode>,
    /// The baked audio plane: interleaved stereo at the master sample rate
    /// over `[0, total_secs]`, produced once at seal. Empty when the tree
    /// has no sound leaves.
    audio: Arc<[f32]>,
    time_marks: Vec<(f64, TimeMark)>,
}

impl SealedRanimScene {
    /// Consume this scene into a [`SceneEvaluator`] driving session.
    ///
    /// `logic_fps` is retained for call-site compatibility; it no longer drives
    /// stepping (each iterative segment owns its own `sim_step`).
    pub fn into_evaluator(self, logic_fps: f64) -> SceneEvaluator {
        SceneEvaluator::new(self, logic_fps)
    }

    /// The baked audio plane: interleaved stereo at the master sample rate
    /// over `[0, total_secs]` (shared, cheap to clone). Empty when the scene
    /// has no sound leaves.
    pub fn audio(&self) -> &Arc<[f32]> {
        &self.audio
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
            .map(AnimNode::animation_info)
            .collect()
    }

    /// Sample all clips active at `target_sec` into typed CPU batches.
    ///
    /// Replaces the whole frame while reusing its batch storage. Each active
    /// leaf contributes its complete output under its top-level animation source.
    pub fn eval_at_sec(&self, target_sec: f64, out: &mut EvaluatedFrame) {
        out.clear();
        for (source_id, animation) in self.animations.iter().enumerate() {
            animation.eval_at(target_sec, &mut OutputContext::new(out, source_id));
        }
    }

    /// Sample by normalized scene progress, replacing `out`.
    pub fn eval_at_alpha(&self, alpha: f64, out: &mut EvaluatedFrame) {
        self.eval_at_sec(self.total_secs * alpha, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{
        build::{PlaybackExt, Unplaced},
        compose::sequence::AnimSequence,
        eval::Static,
    };

    fn leaf(duration: f64) -> impl Unplaced {
        Static(0_u8).with_duration(duration)
    }

    #[test]
    fn scene_play_places_sequences_on_the_root_timeline() {
        // A reusable sequence module keeps its local gaps on the root stack,
        // and later modules start at the root origin.
        let mut reusable = AnimSequence::new();
        reusable.push(leaf(2.0)).forward(1.0).push(leaf(1.0));
        let mut scene = RanimScene::new();
        scene.play(reusable);
        scene.root.push(leaf(5.0));
        let sealed = scene.seal();
        assert_eq!(sealed.total_secs(), 5.0);
        let infos = sealed.get_animation_infos();
        assert_eq!(infos[0].range, 0.0..4.0);
        assert_eq!(infos[0].children[1].range, 3.0..4.0);
        assert_eq!(infos[1].range, 0.0..5.0);
    }

    #[test]
    fn leaf_outputs_keep_containers_whole_while_children_share_source_parts() {
        let mut scene = RanimScene::new();
        scene.play(crate::stack![
            Static(vec![1_u8, 2]).with_duration(1.0),
            Static(3_u8).with_duration(1.0),
            Static((42_u32, String::from("tuple"))).with_duration(1.0),
        ]);
        let sealed = scene.seal();
        let mut frame = EvaluatedFrame::new();
        sealed.eval_at_sec(0.5, &mut frame);

        assert_eq!(frame.len(), 3);
        let vector = &frame.items::<Vec<u8>>()[0];
        assert_eq!(vector.value, [1, 2]);
        assert_eq!((vector.id, vector.order), ((0, 0), 0));
        let item = &frame.items::<u8>()[0];
        assert_eq!(item.value, 3);
        assert_eq!((item.id, item.order), ((0, 1), 1));
        let tuple = &frame.items::<(u32, String)>()[0];
        assert_eq!(tuple.value, (42, String::from("tuple")));
        assert_eq!((tuple.id, tuple.order), ((0, 2), 2));
        assert!(frame.items::<u32>().is_empty());
    }
}
