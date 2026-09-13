//! A lightweight session driver for scene evaluation (no ECS).
//!
//! The [`SceneEvaluator`] owns the lowered animation cells and exposes ONE
//! interaction: [`sample_at`](SceneEvaluator::sample_at), a stateful function
//! of time. Each cell evaluates itself at the target (direction management —
//! forward, backward reset+replay, equal project — is internal to each stateful
//! node), so the session needs no clock bookkeeping beyond remembering the last
//! target.

use crate::{
    Extract, SealedRanimScene, TimeMark,
    animation::node::{AnimNode, AnimationInfo},
    audio::MASTER_SAMPLE_RATE,
    core_item::CoreItem,
};

/// A reusable frame-local output buffer of `((animation_id, part), CoreItem)`.
pub type EvaluatedFrame = Vec<((usize, usize), CoreItem)>;

/// Lightweight scene evaluation session.
pub struct SceneEvaluator {
    cells: Vec<AnimNode>,
    total_secs: f64,
    audio: std::sync::Arc<[f32]>,
    time_marks: Vec<(f64, TimeMark)>,
    clock: f64,
}

impl SceneEvaluator {
    /// Consume a sealed scene and create a driving session.
    ///
    /// `logic_fps` is retained only for call-site compatibility; it no longer
    /// drives stepping (each iterative segment owns its `sim_step`).
    #[allow(unused_variables)]
    pub fn new(scene: SealedRanimScene, logic_fps: f64) -> Self {
        Self {
            cells: scene.animations,
            total_secs: scene.total_secs,
            audio: scene.audio,
            time_marks: scene.time_marks,
            clock: 0.0,
        }
    }

    /// Total scene duration.
    pub fn total_secs(&self) -> f64 {
        self.total_secs
    }

    /// The top-level animation cells.
    ///
    /// Crate-internal: the audio tests walk these with a point-semantics
    /// reference mixer that the seal-time bake must reproduce exactly.
    #[cfg(test)]
    pub(crate) fn cells(&self) -> &[AnimNode] {
        &self.cells
    }

    /// Whether any sound leaves live in the tree.
    pub fn has_audio(&self) -> bool {
        self.cells.iter().any(AnimNode::has_audio)
    }

    /// The baked audio plane: interleaved stereo at the master sample rate
    /// over `[0, total_secs]` (shared, cheap to clone). Empty when the scene
    /// has no sound leaves.
    pub fn audio(&self) -> &std::sync::Arc<[f32]> {
        &self.audio
    }

    /// The scene's audio over `[0, out_secs]` as a fresh interleaved stereo
    /// buffer.
    ///
    /// The audio plane was already mixed once at seal
    /// ([`RanimScene::seal`](crate::RanimScene::seal)); this is a prefix copy
    /// of that baked buffer, zero-padded past the scene end.
    pub fn mix_audio(&self, out_secs: f64, sample_rate: u32) -> Vec<f32> {
        assert_eq!(
            sample_rate, MASTER_SAMPLE_RATE,
            "the baked audio lives at the master sample rate"
        );
        let out_frames = (out_secs * sample_rate as f64).ceil() as usize;
        let baked_frames = self.audio.len() / 2;
        let copy = out_frames.min(baked_frames);
        let mut out = Vec::with_capacity(out_frames * 2);
        out.extend_from_slice(&self.audio[..copy * 2]);
        out.resize(out_frames * 2, 0.0);
        out
    }

    /// Scene time marks.
    pub fn time_marks(&self) -> &[(f64, TimeMark)] {
        &self.time_marks
    }

    /// Hierarchical runtime animation information for preview tooling.
    pub fn animation_infos(&self) -> Vec<AnimationInfo> {
        self.cells.iter().map(AnimNode::animation_info).collect()
    }

    /// Last sampled target (the `clock` reading for preview tooling).
    pub fn clock(&self) -> f64 {
        self.clock
    }

    /// Sample the scene at `render_secs` — the ONLY session interaction.
    ///
    /// Every top-level cell evaluates itself at the target (forward/backward
    /// direction management is internal), and the extracted items carry the
    /// `(animation_id, part)` identities of `SealedRanimScene::eval_at_sec`.
    pub fn sample_at(&mut self, render_secs: f64, out: &mut EvaluatedFrame) {
        for (animation_id, cell) in self.cells.iter().enumerate() {
            let mut items = Vec::new();
            cell.eval_at(render_secs, &mut items);
            for (part, item) in items
                .into_iter()
                .flat_map(|item| item.extract())
                .enumerate()
            {
                out.push(((animation_id, part), item));
            }
        }
        self.clock = render_secs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RanimScene, SealedRanimScene,
        animation::{
            build::{PlaybackExt, Unplaced},
            eval::Eval,
        },
        core_item::vitem::VItem,
        seq,
    };

    /// A constant-velocity iterative segment: x accumulates with progress.
    /// `sim_step` is the content's own step (1/N); `logical_secs` scales it
    /// back to physical seconds. State lives behind a `RefCell` because
    /// `eval_alpha` is a `&self` query.
    struct ConstantVelocity {
        v: f64,
        logical_secs: f64,
        sim_step: f64,
        state: std::cell::RefCell<(f64, f64)>, // (x, alpha)
    }

    impl Eval for ConstantVelocity {
        type Output = VItem;

        fn eval_alpha(&self, target: f64) -> VItem {
            let mut s = self.state.borrow_mut();
            if target < s.1 {
                s.0 = 0.0;
                s.1 = 0.0;
            }
            let start_idx = (s.1 / self.sim_step).floor() as usize;
            let end_idx = (target / self.sim_step).floor() as usize;
            for _ in start_idx..end_idx {
                s.0 += self.v * self.sim_step * self.logical_secs;
            }
            s.1 = target;
            let mut item = VItem::default();
            item.points[0].x = s.0 as f32;
            item
        }
    }

    /// A stateless segment: x = alpha.
    struct ProgressX;

    impl Eval for ProgressX {
        type Output = VItem;

        fn eval_alpha(&self, alpha: f64) -> VItem {
            let mut item = VItem::default();
            item.points[0].x = alpha as f32;
            item
        }
    }

    fn xs_of(frame: &EvaluatedFrame) -> Vec<f32> {
        frame
            .iter()
            .filter_map(|(_, item)| match item {
                CoreItem::VItem(v) => Some(v.points[0].x),
                _ => None,
            })
            .collect()
    }

    fn cv(v: f64, logical_secs: f64) -> ConstantVelocity {
        ConstantVelocity {
            v,
            logical_secs,
            sim_step: 1.0 / 120.0,
            state: std::cell::RefCell::new((0.0, 0.0)),
        }
    }

    #[test]
    fn functional_scene_matches_pure_eval() {
        let mut scene = RanimScene::new();
        scene.play(ProgressX.with_duration(2.0));
        let sealed = scene.seal();

        let mut ev = SceneEvaluator::new(sealed, 120.0);
        for sec in [0.0, 0.25, 0.5, 1.0, 1.5, 2.0] {
            let mut frame = EvaluatedFrame::new();
            ev.sample_at(sec, &mut frame);
            let expected = (sec / 2.0) as f32;
            let got = xs_of(&frame);
            assert_eq!(got, vec![expected], "at sec={sec}");
        }
    }

    #[test]
    fn iterative_leaves_step_along_the_logic_grid() {
        let mut scene = RanimScene::new();
        scene.play(cv(1.0, 2.0).with_duration(2.0).at(0.0));
        let mut ev = SceneEvaluator::new(scene.seal(), 120.0);
        for sec in [0.0, 0.5, 1.0, 1.5, 2.0] {
            let mut frame = EvaluatedFrame::new();
            ev.sample_at(sec, &mut frame);
            assert_eq!(xs_of(&frame), vec![sec as f32], "at sec={sec}");
        }

        // Nested inside a sequence the leaf still steps on its own timeline.
        let mut scene = RanimScene::new();
        scene.play(
            seq![
                cv(1.0, 1.0).with_duration(1.0),
                cv(1.0, 1.0).with_duration(1.0)
            ]
            .at(0.0),
        );
        let mut ev = SceneEvaluator::new(scene.seal(), 120.0);
        for (sec, expected) in [(1.5, 0.5), (2.0, 1.0)] {
            let mut frame = EvaluatedFrame::new();
            ev.sample_at(sec, &mut frame);
            assert_eq!(xs_of(&frame), vec![expected], "at sec={sec}");
        }
    }

    #[test]
    fn iterative_seek_matches_forward_and_resets_nested_leaves() {
        fn run(scene: SealedRanimScene, backward: bool) -> Vec<Vec<f32>> {
            let mut ev = SceneEvaluator::new(scene, 120.0);
            let mut trace = Vec::new();
            for sec in [0.3, 0.7, 1.1, 1.9, 2.6] {
                if backward {
                    // Jump backwards below the first sample so every leaf
                    // has to re-simulate from the start.
                    ev.sample_at(0.2, &mut EvaluatedFrame::new());
                }
                let mut frame = EvaluatedFrame::new();
                ev.sample_at(sec, &mut frame);
                trace.push(xs_of(&frame));
            }
            trace
        }

        let single = || {
            let mut scene = RanimScene::new();
            scene.play(cv(2.0, 3.0).with_duration(3.0).at(0.0));
            scene.seal()
        };
        let forward = run(single(), false);
        assert_eq!(forward, run(single(), true));
        assert_eq!(forward[2], vec![(2.0 * 1.1) as f32]);

        let nested = || {
            let mut scene = RanimScene::new();
            scene.play(
                seq![
                    cv(1.0, 1.0).with_duration(1.0),
                    cv(1.0, 1.0).with_duration(1.0)
                ]
                .at(0.0),
            );
            scene.seal()
        };
        assert_eq!(run(nested(), false), run(nested(), true));
    }
}
