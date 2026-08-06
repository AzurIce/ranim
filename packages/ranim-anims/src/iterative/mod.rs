//! Iterative (stateful, stepped) evaluation: the
//! [`IterativeEval`](crate::iterative::IterativeEval) capability trait and its
//! [`Iterative`](crate::iterative::Iterative) adapter into the general
//! [`Eval`](ranim_core::animation::Eval) protocol.

use ranim_core::{
    animation::Eval,
    time::{DeltaTime, Time},
};

/// The capability of an iterative, stateful evaluation.
///
/// This is what iterative animation types implement: particles, springs,
/// physics simulations, and anything without a closed form. All three methods
/// are required — the compiler enforces `reset`, so replay determinism
/// ([`SceneEvaluator::seek`](ranim_core::SceneEvaluator::seek)) cannot be
/// silently broken by omission.
///
/// How long the segment simulates is a construction parameter of the segment
/// itself; `with_duration` is only a playback stretch. `step` receives the
/// step length as `delta_time.alpha` (in warped local progress) — scale it by
/// the segment's own logical duration to recover meaningful units:
///
/// ```rust,ignore
/// fn step(&mut self, _time: &Time, delta_time: &DeltaTime) {
///     let dt = self.sim_secs * delta_time.alpha; // logical seconds
///     // integrate with dt ...
/// }
/// ```
///
/// Segments that need unwarped wall-clock-shaped time use
/// `time.global_secs`/`delta_time.global_secs` instead.
///
/// Wrap an `IterativeEval` in [`Iterative`] to turn it into a full animation
/// segment.
pub trait IterativeEval {
    /// Value produced by this evaluator.
    type Output;

    /// Project the current internal state into the output.
    fn sample(&self) -> Self::Output;

    /// Reset to the segment's initial state (deterministic contract: no wall
    /// clock, no unseeded RNG).
    fn reset(&mut self);

    /// Advance one logic step.
    fn step(&mut self, time: &Time, delta_time: &DeltaTime);
}

/// Adapter turning an [`IterativeEval`] into the general [`Eval`] protocol.
///
/// An iterative segment is nearly the general case: `sample` projects the
/// current state (the time point is routing information, dropped here), and
/// `reset`/`step` pass straight through.
///
/// ```rust,ignore
/// let animation = Iterative(NBody::new(99, 32.0)).with_duration(32.0);
/// ```
pub struct Iterative<E>(pub E);

impl<E: IterativeEval> Eval for Iterative<E> {
    type Output = E::Output;

    fn sample(&self, _time: &Time) -> Self::Output {
        self.0.sample()
    }

    fn reset(&mut self) {
        self.0.reset();
    }

    fn step(&mut self, time: &Time, delta_time: &DeltaTime) {
        self.0.step(time, delta_time);
    }
}
