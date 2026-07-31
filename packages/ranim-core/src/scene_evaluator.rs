//! A lightweight session driver for scene evaluation (no ECS).
//!
//! The [`SceneEvaluator`] owns the lowered animation cells and drives them
//! along a fixed logic grid:
//!
//! - [`SceneEvaluator::advance_to`] advances the internal clock to the floor
//!   logic tick of a render sample time, stepping every active cell (iterative
//!   leaves integrate; functional leaves step as no-ops);
//! - [`SceneEvaluator::sample_into`] is pure: it samples every active cell at
//!   the internal clock and extracts `CoreItem`s, mirroring
//!   [`SealedRanimScene::eval_at_sec`]'s `(animation_id, part)` identities;
//! - [`SceneEvaluator::seek`] resets all cells and replays (deterministic
//!   contract: replay equals forward advancement).
//!
//! This is the M1 runtime: self-contained iterative animations work without
//! ECS. See `docs`/design notes for the surrounding design.

use crate::{
    Extract, SealedRanimScene, TimeMark,
    animation::{AnimationCell, AnimationInfo},
    core_item::CoreItem,
};

/// A reusable frame-local output buffer of `((animation_id, part), CoreItem)`.
pub type EvaluatedFrame = Vec<((usize, usize), CoreItem)>;

/// Lightweight scene evaluation session.
pub struct SceneEvaluator {
    cells: Vec<AnimationCell>,
    total_secs: f64,
    time_marks: Vec<(f64, TimeMark)>,
    logic_fps: f64,
    clock: f64,
}

impl SceneEvaluator {
    /// Consume a sealed scene and create a driving session.
    ///
    /// `logic_fps` is the fixed logic grid resolution (`1 / logic_fps` is the
    /// integration step). The time model defaults to 120 Hz; render fps only
    /// decides which logic states are sampled.
    pub fn new(scene: SealedRanimScene, logic_fps: f64) -> Self {
        assert!(
            logic_fps.is_finite() && logic_fps > 0.0,
            "logic_fps must be finite and positive"
        );
        Self {
            cells: scene.animations,
            total_secs: scene.total_secs,
            time_marks: scene.time_marks,
            logic_fps,
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

    /// Current internal clock (the last advanced floor logic tick).
    pub fn clock(&self) -> f64 {
        self.clock
    }

    /// Advance the logic grid to the floor tick of `render_secs`.
    ///
    /// This is the only entry point that performs tick advancement. Calling it
    /// with a time earlier than the current clock is a no-op; use
    /// [`seek`](Self::seek) to move backwards.
    pub fn advance_to(&mut self, render_secs: f64) {
        let target = (render_secs * self.logic_fps).floor() / self.logic_fps;
        while self.clock + 1e-9 < target {
            let prev_tick_secs = self.clock;
            let tick_secs = prev_tick_secs + 1.0 / self.logic_fps;
            self.clock = tick_secs;
            for cell in &mut self.cells {
                cell.step_at_sec(tick_secs, prev_tick_secs);
            }
        }
        self.clock = target;
    }

    /// Reset all cells and replay to `render_secs` (deterministic contract).
    pub fn seek(&mut self, render_secs: f64) {
        self.clock = 0.0;
        for cell in &mut self.cells {
            cell.reset_entered();
        }
        self.advance_to(render_secs);
    }

    /// Pure sampling of the current clock: no tick advancement.
    ///
    /// Extracts every active cell at the floor logic tick into `out` with
    /// `(animation_id, part)` identities matching `SealedRanimScene::eval_at_sec`.
    /// Call [`advance_to`](Self::advance_to) or [`seek`](Self::seek) first.
    pub fn sample_into(&self, out: &mut EvaluatedFrame) {
        for (animation_id, cell) in self.cells.iter().enumerate() {
            if !cell.active_at(self.clock) {
                continue;
            }
            let mut items = Vec::new();
            cell.sample_at_sec(self.clock, &mut items);
            for (part, item) in items
                .into_iter()
                .flat_map(|item| item.extract())
                .enumerate()
            {
                out.push(((animation_id, part), item));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RanimScene,
        animation::{AnimationExt, Evaluator, Placeable, SegmentTime},
        core_item::vitem::VItem,
        seq,
    };

    /// A constant-velocity iterative segment: x accumulates with local time.
    struct ConstantVelocity {
        v: f64,
        x: f64,
    }

    impl Evaluator for ConstantVelocity {
        type Output = VItem;

        fn sample(&self, _time: &SegmentTime) -> VItem {
            let mut item = VItem::default();
            item.points[0].x = self.x as f32;
            item
        }

        fn reset(&mut self) {
            self.x = 0.0;
        }

        fn step(&mut self, time: &SegmentTime) {
            self.x += self.v * time.local_delta_secs;
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

    #[test]
    fn functional_scene_matches_pure_eval() {
        let mut scene = RanimScene::new();
        scene.play(
            (move |alpha| {
                let mut item = VItem::default();
                item.points[0].x = alpha as f32;
                item
            })
            .with_duration(2.0),
        );
        let sealed = scene.seal();

        let mut ev = SceneEvaluator::new(sealed, 120.0);
        for sec in [0.0, 0.25, 0.5, 1.0, 1.5, 2.0] {
            let mut frame = EvaluatedFrame::new();
            ev.advance_to(sec);
            ev.sample_into(&mut frame);
            // Pure path value at the same time (rate = linear, alpha = sec/2)
            let expected = (sec / 2.0) as f32;
            let got = xs_of(&frame);
            assert_eq!(got, vec![expected], "at sec={sec}");
        }
    }

    #[test]
    fn iterative_segment_steps_along_logic_grid() {
        let mut scene = RanimScene::new();
        scene.play(
            ConstantVelocity { v: 1.0, x: 0.0 }
                .with_duration(2.0)
                .at(0.0),
        );
        let mut ev = SceneEvaluator::new(scene.seal(), 120.0);

        for sec in [0.0, 0.5, 1.0, 1.5, 2.0] {
            let mut frame = EvaluatedFrame::new();
            ev.advance_to(sec);
            ev.sample_into(&mut frame);
            assert_eq!(xs_of(&frame), vec![sec as f32], "at sec={sec}");
        }
    }

    #[test]
    fn iterative_eval_is_deterministic_and_seek_matches_forward() {
        fn build_scene() -> SealedRanimScene {
            let mut scene = RanimScene::new();
            scene.play(
                ConstantVelocity { v: 2.0, x: 0.0 }
                    .with_duration(3.0)
                    .at(0.0),
            );
            scene.seal()
        }

        let run = |seek_first: bool| {
            let mut ev = SceneEvaluator::new(build_scene(), 120.0);
            let mut trace = Vec::new();
            for sec in [0.3, 0.7, 1.1, 1.9, 2.6] {
                if seek_first {
                    ev.seek(sec);
                } else {
                    ev.advance_to(sec);
                }
                let mut frame = EvaluatedFrame::new();
                ev.sample_into(&mut frame);
                trace.push(xs_of(&frame));
            }
            trace
        };

        let forward = run(false);
        let seek = run(true);
        assert_eq!(forward, seek);
        // Matches the analytic value: x = v·t
        assert_eq!(forward[2], vec![(2.0 * 1.1) as f32]);
    }

    #[test]
    fn iterative_leaf_inside_sequence_steps() {
        let mut scene = RanimScene::new();
        scene.play(
            seq![
                ConstantVelocity { v: 1.0, x: 0.0 }.with_duration(1.0),
                ConstantVelocity { v: 1.0, x: 0.0 }.with_duration(1.0),
            ]
            .at(0.0),
        );

        let mut ev = SceneEvaluator::new(scene.seal(), 120.0);
        // t=1.5: the first is done (under sequence semantics it no longer appears
        // in the frame, matching the pure path); the second is at 0.5 (x=0.5)
        let mut frame = EvaluatedFrame::new();
        ev.advance_to(1.5);
        ev.sample_into(&mut frame);
        assert_eq!(xs_of(&frame), vec![0.5]);

        // Final state: the second completes with x=1.0
        let mut frame = EvaluatedFrame::new();
        ev.advance_to(2.0);
        ev.sample_into(&mut frame);
        assert_eq!(xs_of(&frame), vec![1.0]);
    }
}
