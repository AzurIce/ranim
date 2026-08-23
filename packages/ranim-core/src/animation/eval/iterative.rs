//! Iterative (stateful, stepped) evaluation: the
//! [`IterativeEval`](crate::animation::eval::iterative::IterativeEval)
//! capability trait and its
//! [`Iterative`](crate::animation::eval::iterative::Iterative) adapter into the
//! general [`Eval`] protocol.
//!
//! **Content is sequence**: an iterative segment owns its simulation step
//! (`sim_step`, declared via `with_steps(N)`), its integration state, and its
//! current progress. Advancing folds direction management — forward integrates
//! `sim_step`-by-`sim_step`, backward resets and replays — so the runtime
//! only ever asks "evaluate at progress `alpha`". No seconds, no scene clock,
//! no `logic_fps` reach a segment's content.
//!
//! If `with_steps` is not called, the segment uses the default step of
//! `1 / 120` (`DEFAULT_SIM_STEP`): every unit of normalized progress is integrated in 120 uniform
//! substeps. Use `with_steps(N)` when a simulation needs a finer or coarser
//! content resolution.

use std::{cell::RefCell, marker::PhantomData};

use super::Eval;

/// The capability of an iterative, stateful evaluation.
///
/// This is what iterative animation types implement: particles, springs,
/// physics simulations, and anything without a closed form. The state is owned
/// and advanced by the [`Iterative`] adapter, and `step` receives it as a
/// mutable reference.
///
/// Only one method, no defaults. There is no `reset` to forget or get wrong —
/// the adapter restores the stored initial state itself. Physics parameters,
/// palettes, and logical durations belong on `self` or in local variables
/// captured by a closure (not in global constants); everything mutable must
/// live in the state value, so a reset restores it all.
///
/// `step` receives the current progress `alpha` and the segment's own
/// uniform progress step `delta_alpha` (`= sim_step = 1 / N`). These are
/// **dimensionless progress**, independent of rate shaping and placement; the
/// segment's content is a pure function of progress. To recover physical
/// seconds, scale by the segment's own logical duration:
///
/// ```rust,ignore
/// fn step(&self, state: &mut NBodyState, _alpha: f64, delta_alpha: f64) {
///     let dt = self.sim_secs * delta_alpha; // physical seconds
///     state.integrate(dt);
/// }
/// ```
///
/// Segments that genuinely need scene-clock-shaped time must be authored at the
/// top level, where the author knows the placement.
pub trait IterativeEval {
    /// State produced and advanced by this evaluator.
    type Output;

    /// Advance the state by one progress step.
    ///
    /// `alpha` is the current progress, `delta_alpha` the segment's uniform
    /// step (both dimensionless progress). The step size is declared on the
    /// [`Iterative`] adapter with
    /// [`Iterative::with_steps`]: `delta_alpha = 1 / N` for `with_steps(N)`,
    /// or the default `1 / 120` when it is not called.
    fn step(&self, output: &mut Self::Output, alpha: f64, delta_alpha: f64);
}

/// An [`IterativeEval`] backed by a stepping function.
///
/// The wrapper binds the function's mutable input type as the evaluator's
/// unique [`Output`](IterativeEval::Output). Prefer [`Iterative::from_fn`] to
/// constructing this type directly.
pub struct IterativeFn<S, F> {
    eval: F,
    output: PhantomData<fn() -> S>,
}

impl<S, F> IterativeFn<S, F>
where
    F: Fn(&mut S, f64, f64),
{
    /// Bind a stepping function to its state type.
    pub fn new(eval: F) -> Self {
        Self {
            eval,
            output: PhantomData,
        }
    }
}

impl<S, F> IterativeEval for IterativeFn<S, F>
where
    F: Fn(&mut S, f64, f64),
{
    type Output = S;

    fn step(&self, output: &mut Self::Output, alpha: f64, delta_alpha: f64) {
        (self.eval)(output, alpha, delta_alpha)
    }
}

/// The default content step when the author does not declare one: 1/120
/// progress per step (the historical 120 Hz logic-grid resolution, now
/// expressed as a per-segment content property rather than a scene parameter).
const DEFAULT_SIM_STEP: f64 = 1.0 / 120.0;

/// The memoization snapshot backing an iterative segment.
///
/// Holds the progress already reached (`alpha`) and the state there. Because
/// `eval_alpha` is a **pure query** on `&self` — an animation's content is
/// immutable once defined — the adapter keeps this snapshot behind a
/// `RefCell`: advancing writes into it, projecting reads from it, and neither
/// escapes the `&self` query contract. This is memoization, not mutation of
/// the animation's definition.
struct Snapshot<S> {
    alpha: f64,
    state: S,
}

/// Adapter turning an [`IterativeEval`] into the general [`Eval`] protocol.
///
/// The adapter owns the segment's definition — the initial state, the `sim_step`,
/// and the stepping closure — all immutable. The only mutation is the snapshot
/// cache (`alpha` + `state`) behind a `RefCell`, so `eval_alpha(target)`
/// integrates to `target` only when `target` is ahead of the snapshot, resets
/// and replays when behind, and otherwise returns the cached state directly.
/// Repeated queries at the same `alpha` are O(1).
///
/// ```rust,ignore
/// let sim_secs = 10.0;
/// let animation = Iterative::from_fn(state0, move |state, _alpha, delta_alpha| {
///     state.integrate(sim_secs * delta_alpha);
/// })
/// .with_steps(240)
/// .with_duration(sim_secs);
/// ```
pub struct Iterative<E: IterativeEval> {
    eval: E,
    initial: E::Output,
    sim_step: f64,
    snapshot: RefCell<Snapshot<E::Output>>,
}

impl<E> Iterative<E>
where
    E: IterativeEval,
    E::Output: Clone,
{
    /// Create an iterative segment from an initial state and a named
    /// [`IterativeEval`] implementation.
    ///
    /// The content step defaults to `1 / 120` of normalized progress. Call
    /// [`with_steps`](Self::with_steps) to override it, e.g.
    /// `.with_steps(240)` for `1 / 240` progress increments.
    pub fn new(initial: E::Output, eval: E) -> Self {
        Self {
            eval,
            snapshot: RefCell::new(Snapshot {
                alpha: 0.0,
                state: initial.clone(),
            }),
            initial,
            sim_step: DEFAULT_SIM_STEP,
        }
    }

    /// Declare the content's step count `N`.
    ///
    /// The segment then integrates in uniform `1 / N` progress increments, so
    /// every [`IterativeEval::step`] call receives `delta_alpha = 1 / N`.
    /// Without this call the default is `1 / 120` (`DEFAULT_SIM_STEP`).
    ///
    /// This is the segment's **content resolution**, not the render sampling
    /// rate and not the scene clock. A physical time step can be recovered as
    /// `duration_secs * delta_alpha`.
    ///
    /// ```rust,ignore
    /// Iterative::from_fn(initial_state, step_function)
    ///     .with_steps(240) // 1/240 progress per integration step
    ///     .with_duration(10.0); // => 1/24 seconds per integration step
    /// ```
    pub fn with_steps(mut self, n: usize) -> Self {
        assert!(n > 0, "iterative step count must be positive");
        self.sim_step = 1.0 / n as f64;
        self
    }
}

impl<S, F> Iterative<IterativeFn<S, F>>
where
    S: Clone,
    F: Fn(&mut S, f64, f64),
{
    /// Create an iterative segment from an initial state and a stepping
    /// function.
    ///
    /// Uses the default `1 / 120` content step; call
    /// [`with_steps`](Iterative::with_steps) to choose another resolution.
    pub fn from_fn(initial: S, eval: F) -> Self {
        Self::new(initial, IterativeFn::new(eval))
    }
}

impl<E> Eval for Iterative<E>
where
    E: IterativeEval,
    E::Output: Clone,
{
    type Output = E::Output;

    fn sim_step(&self) -> Option<f64> {
        Some(self.sim_step)
    }

    fn eval_alpha(&self, target: f64) -> Self::Output {
        let mut snap = self.snapshot.borrow_mut();
        if target < snap.alpha {
            // Backward target: reset and replay from the start.
            snap.state = self.initial.clone();
            snap.alpha = 0.0;
        }
        // Integrate by whole steps (index-based: no floating-point drift).
        let start_idx = (snap.alpha / self.sim_step).floor() as usize;
        let end_idx = (target / self.sim_step).floor() as usize;
        for i in start_idx..end_idx {
            let alpha = i as f64 * self.sim_step;
            self.eval.step(&mut snap.state, alpha, self.sim_step);
        }
        snap.alpha = target;
        snap.state.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RanimScene, SceneEvaluator,
        animation::AnimationExt,
        core_item::{CoreItem, vitem::VItem},
    };

    struct MoveRight;

    impl IterativeEval for MoveRight {
        type Output = VItem;

        fn step(&self, state: &mut Self::Output, _alpha: f64, delta_alpha: f64) {
            state.points[0].x += delta_alpha as f32;
        }
    }

    #[test]
    fn named_evaluator_advances_by_sim_step() {
        let animation = Iterative::new(VItem::default(), MoveRight).with_steps(4);
        assert_eq!(animation.eval_alpha(0.5).points[0].x, 0.5);
    }

    /// A closure-driven iterative segment with a directly renderable state:
    /// stepping and seek replay must be deterministic.
    #[test]
    fn closure_segment_steps_and_seeks_deterministically() {
        fn build() -> SceneEvaluator {
            let mut scene = RanimScene::new();
            scene.play(
                Iterative::from_fn(
                    VItem::default(),
                    |state: &mut VItem, _alpha: f64, delta_alpha: f64| {
                        state.points[0].x += delta_alpha as f32;
                    },
                )
                .with_steps(240)
                .with_duration(2.0),
            );
            scene.seal().into_evaluator()
        }

        let run = || {
            let mut ev = build();
            let mut trace = Vec::new();
            for sec in [0.5, 1.0, 1.5, 2.0] {
                let mut frame = Vec::new();
                ev.sample_at(sec, &mut frame);
                let x = frame.iter().find_map(|(_, item)| match item {
                    CoreItem::VItem(v) => Some(v.points[0].x),
                    _ => None,
                });
                trace.push(x);
            }
            trace
        };

        let forward = run();
        assert_eq!(forward, run());
        let x = forward.last().unwrap().unwrap();
        assert!((x - 1.0).abs() < 1e-3, "x = {x}");
    }
}
