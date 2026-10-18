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
    frame::EvaluatedFrame,
    lifetime::KeepAlive,
};

/// Lightweight scene evaluation session.
pub struct SceneEvaluator {
    cells: Vec<AnimNode>,
    total_secs: f64,
    audio: std::sync::Arc<[f32]>,
    time_marks: Vec<(f64, TimeMark)>,
    clock: f64,
    // Keep external animation code loaded until all cells have been destroyed.
    _keep_alive: KeepAlive,
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
            _keep_alive: scene.keep_alive,
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
    /// Replaces `out` with the new sample while reusing its batch storage.
    pub fn sample_at(&mut self, render_secs: f64, out: &mut EvaluatedFrame) {
        out.update(
            self.cells
                .iter()
                .enumerate()
                .flat_map(|(animation_id, cell)| {
                    let mut items = Vec::new();
                    cell.eval_at(render_secs, &mut items);
                    items
                        .into_iter()
                        .flat_map(|item| item.extract())
                        .enumerate()
                        .map(move |(part, item)| ((animation_id, part), item))
                }),
        );
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
            eval::{Eval, iterative::Iterative},
        },
        core_item::{CoreItem, vitem::VItem},
        seq,
    };

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
            .items::<CoreItem>()
            .iter()
            .filter_map(|item| match &item.value {
                CoreItem::VItem(v) => Some(v.points[0].x),
                _ => None,
            })
            .collect()
    }

    fn cv(v: f64, logical_secs: f64) -> impl Unplaced {
        Iterative::from_fn(VItem::default(), move |state: &mut VItem, _, delta| {
            state.points[0].x += (v * logical_secs * delta) as f32;
        })
        .with_steps(120)
    }

    fn assert_x(frame: &EvaluatedFrame, expected: f32) {
        let values = xs_of(frame);
        assert_eq!(values.len(), 1);
        assert!(
            (values[0] - expected).abs() < 1e-5,
            "{values:?} vs {expected}"
        );
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
    fn repeated_samples_replace_the_whole_output_frame() {
        let mut scene = RanimScene::new();
        scene.play(ProgressX.with_duration(2.0));
        let mut ev = SceneEvaluator::new(scene.seal(), 120.0);
        let mut frame = EvaluatedFrame::new();
        frame.push((99, 0), String::from("previous output"));

        for sec in [0.5, 1.5, 0.25, 1.5] {
            ev.sample_at(sec, &mut frame);

            assert_eq!(frame.len(), 1);
            assert!(frame.items::<String>().is_empty());
            let items = frame.items::<CoreItem>();
            assert_eq!(items[0].id, (0, 0));
            assert_eq!(items[0].order, 0);
            assert_x(&frame, (sec / 2.0) as f32);
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
            assert_x(&frame, sec as f32);
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
            assert_x(&frame, expected);
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
        assert!((forward[2][0] - 2.2).abs() < 1e-5);

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
