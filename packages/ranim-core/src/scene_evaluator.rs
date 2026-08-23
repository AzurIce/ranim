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
    animation::{AnimationCell, AnimationInfo},
    core_item::CoreItem,
};

/// A reusable frame-local output buffer of `((animation_id, part), CoreItem)`.
pub type EvaluatedFrame = Vec<((usize, usize), CoreItem)>;

/// A scene driving session: the common interface of the pure evaluator
/// ([`SceneEvaluator`]) and the retained-world player
/// ([`ScenePlayer`](crate::ScenePlayer)).
///
/// Evaluation is a pure query (`eval_alpha`), so a session needs no stepping
/// or seek bookkeeping: [`sample_at`](SceneSession::sample_at) is the only
/// interaction, and direction management is internal to each stateful node.
/// The two implementations produce identical frames for the same target,
/// so consumers can treat them interchangeably.
pub trait SceneSession {
    /// Consume a sealed scene and create a driving session.
    fn from_sealed(scene: SealedRanimScene) -> Self;

    /// Total scene duration.
    fn total_secs(&self) -> f64;

    /// Last sampled target (the `clock` reading for preview tooling).
    fn clock(&self) -> f64;

    /// Scene time marks.
    fn time_marks(&self) -> &[(f64, TimeMark)];

    /// Hierarchical runtime animation information for preview tooling.
    fn animation_infos(&self) -> Vec<AnimationInfo>;

    /// Sample the scene at `render_secs` into `out` — the ONLY session
    /// interaction. `out` is appended to; clearing it between frames is the
    /// caller's responsibility.
    fn sample_at(&mut self, render_secs: f64, out: &mut EvaluatedFrame);
}

/// Lightweight scene evaluation session.
pub struct SceneEvaluator {
    cells: Vec<AnimationCell>,
    total_secs: f64,
    time_marks: Vec<(f64, TimeMark)>,
    clock: f64,
}

impl SceneEvaluator {
    /// Consume a sealed scene and create a driving session.
    pub fn new(scene: SealedRanimScene) -> Self {
        Self {
            cells: scene.animations,
            total_secs: scene.total_secs,
            time_marks: scene.time_marks,
            clock: 0.0,
        }
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
    pub fn animation_infos(&self) -> Vec<AnimationInfo> {
        self.cells
            .iter()
            .map(AnimationCell::animation_info)
            .collect()
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

impl SceneSession for SceneEvaluator {
    fn from_sealed(scene: SealedRanimScene) -> Self {
        Self::new(scene)
    }

    fn total_secs(&self) -> f64 {
        self.total_secs()
    }

    fn clock(&self) -> f64 {
        self.clock()
    }

    fn time_marks(&self) -> &[(f64, TimeMark)] {
        self.time_marks()
    }

    fn animation_infos(&self) -> Vec<AnimationInfo> {
        self.animation_infos()
    }

    fn sample_at(&mut self, render_secs: f64, out: &mut EvaluatedFrame) {
        self.sample_at(render_secs, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RanimScene, SealedRanimScene,
        animation::{AnimationExt, Placeable, eval::Eval},
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

        let mut ev = SceneEvaluator::new(sealed);
        for sec in [0.0, 0.25, 0.5, 1.0, 1.5, 2.0] {
            let mut frame = EvaluatedFrame::new();
            ev.sample_at(sec, &mut frame);
            let expected = (sec / 2.0) as f32;
            let got = xs_of(&frame);
            assert_eq!(got, vec![expected], "at sec={sec}");
        }
    }

    #[test]
    fn iterative_segment_steps_along_logic_grid() {
        let mut scene = RanimScene::new();
        scene.play(cv(1.0, 2.0).with_duration(2.0).at(0.0));
        let mut ev = SceneEvaluator::new(scene.seal());

        for sec in [0.0, 0.5, 1.0, 1.5, 2.0] {
            let mut frame = EvaluatedFrame::new();
            ev.sample_at(sec, &mut frame);
            assert_eq!(xs_of(&frame), vec![sec as f32], "at sec={sec}");
        }
    }

    #[test]
    fn iterative_eval_is_deterministic_and_seek_matches_forward() {
        fn build_scene() -> SealedRanimScene {
            let mut scene = RanimScene::new();
            scene.play(cv(2.0, 3.0).with_duration(3.0).at(0.0));
            scene.seal()
        }

        let run = |backward: bool| {
            let mut ev = SceneEvaluator::new(build_scene());
            let mut trace = Vec::new();
            for sec in [0.3, 0.7, 1.1, 1.9, 2.6] {
                if backward {
                    ev.sample_at(0.4, &mut EvaluatedFrame::new()); // backward jump
                }
                let mut frame = EvaluatedFrame::new();
                ev.sample_at(sec, &mut frame);
                trace.push(xs_of(&frame));
            }
            trace
        };

        assert_eq!(run(false), run(true));
        assert_eq!(run(false)[2], vec![(2.0 * 1.1) as f32]);
    }

    #[test]
    fn iterative_leaf_inside_sequence_steps() {
        let mut scene = RanimScene::new();
        scene.play(
            seq![
                cv(1.0, 1.0).with_duration(1.0),
                cv(1.0, 1.0).with_duration(1.0)
            ]
            .at(0.0),
        );

        let mut ev = SceneEvaluator::new(scene.seal());
        let mut frame = EvaluatedFrame::new();
        ev.sample_at(1.5, &mut frame);
        assert_eq!(xs_of(&frame), vec![0.5]);

        let mut frame = EvaluatedFrame::new();
        ev.sample_at(2.0, &mut frame);
        assert_eq!(xs_of(&frame), vec![1.0]);
    }

    #[test]
    fn seek_resets_iterative_leaves_nested_in_containers() {
        fn build_scene() -> SealedRanimScene {
            let mut scene = RanimScene::new();
            scene.play(
                seq![
                    cv(1.0, 1.0).with_duration(1.0),
                    cv(1.0, 1.0).with_duration(1.0)
                ]
                .at(0.0),
            );
            scene.seal()
        }

        let run = |backward: bool| {
            let mut ev = SceneEvaluator::new(build_scene());
            let mut trace = Vec::new();
            for sec in [0.3, 0.7, 1.1, 1.5, 1.9] {
                if backward {
                    ev.sample_at(0.2, &mut EvaluatedFrame::new()); // backward jump
                }
                let mut frame = EvaluatedFrame::new();
                ev.sample_at(sec, &mut frame);
                trace.push(xs_of(&frame));
            }
            trace
        };

        assert_eq!(run(false), run(true));
    }
}
