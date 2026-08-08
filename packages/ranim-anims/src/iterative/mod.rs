//! Iterative (stateful, stepped) evaluation: the
//! [`IterativeEval`](crate::iterative::IterativeEval) capability trait and its
//! [`Iterative`](crate::iterative::Iterative) adapter into the general
//! [`Eval`](ranim_core::animation::Eval) protocol.

use std::marker::PhantomData;

use ranim_core::{
    animation::Eval,
    time::{DeltaTime, Time},
};

/// The capability of an iterative, stateful evaluation.
///
/// This is what iterative animation types implement: particles, springs,
/// physics simulations, and anything without a closed form. The state is owned
/// and advanced by the [`Iterative`] adapter, and `step` receives it as a
/// mutable reference.
///
/// Only one method, no defaults. There is no `reset` to forget or get wrong —
/// the adapter restores the stored initial state itself. Constants (physics
/// parameters, palettes, ...) belong in `self` or closure captures; everything
/// mutable must live in the state value, so a reset restores it all.
///
/// How long the segment simulates is part of the step logic's own parameters;
/// `with_duration` is only a playback stretch. `step` receives the step length
/// as `delta_time.alpha` (in warped local progress) — scale it by the
/// segment's own logical duration to recover meaningful units:
///
/// ```rust,ignore
/// fn step(&self, state: &mut NBodyState, _time: &Time, delta_time: &DeltaTime) {
///     let dt = self.sim_secs * delta_time.alpha; // logical seconds
///     // integrate with dt ...
/// }
/// ```
///
/// Segments that need unwarped wall-clock-shaped time use
/// `time.global_secs`/`delta_time.global_secs` instead.
pub trait IterativeEval {
    /// State produced and advanced by this evaluator.
    type Output;

    /// Advance the state by one logic step.
    fn step(&self, output: &mut Self::Output, time: &Time, delta_time: &DeltaTime);
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
    F: Fn(&mut S, &Time, &DeltaTime),
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
    F: Fn(&mut S, &Time, &DeltaTime),
{
    type Output = S;

    fn step(&self, output: &mut Self::Output, time: &Time, delta_time: &DeltaTime) {
        (self.eval)(output, time, delta_time)
    }
}

/// Adapter turning an [`IterativeEval`] into the general [`Eval`] protocol.
///
/// The adapter owns the segment's state: it samples by cloning the current
/// state, and resets by restoring the stored initial state — both structural,
/// nothing for the author to implement or get wrong. When the state differs
/// from what should be rendered, implement [`Extract`](ranim_core::Extract)
/// for the state type: extraction is the per-frame projection point.
///
/// ```rust,ignore
/// // With a closure and a directly-renderable state:
/// let animation = Iterative::from_fn(state0, |state: &mut MyState, _t, dt| {
///     state.integrate(dt.alpha * SIM_SECS);
/// });
///
/// // With a named step type:
/// let animation = Iterative::new(NBodyState::regular_ngon(99), NBodyStep { sim_secs: 32.0 });
/// ```
pub struct Iterative<E: IterativeEval> {
    initial: E::Output,
    current: E::Output,
    eval: E,
}

impl<E> Iterative<E>
where
    E: IterativeEval,
    E::Output: Clone,
{
    /// Create an iterative segment from an initial state and a named
    /// [`IterativeEval`] implementation.
    pub fn new(initial: E::Output, eval: E) -> Self {
        Self {
            current: initial.clone(),
            initial,
            eval,
        }
    }
}

impl<S, F> Iterative<IterativeFn<S, F>>
where
    S: Clone,
    F: Fn(&mut S, &Time, &DeltaTime),
{
    /// Create an iterative segment from an initial state and a stepping
    /// function.
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

    fn sample(&self, _time: &Time) -> Self::Output {
        self.current.clone()
    }

    fn reset(&mut self) {
        self.current = self.initial.clone();
    }

    fn step(&mut self, time: &Time, delta_time: &DeltaTime) {
        self.eval.step(&mut self.current, time, delta_time);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ranim_core::{
        RanimScene, SceneEvaluator,
        animation::AnimationExt,
        core_item::{CoreItem, vitem::VItem},
    };

    struct MoveRight;

    impl IterativeEval for MoveRight {
        type Output = VItem;

        fn step(&self, state: &mut Self::Output, _time: &Time, delta_time: &DeltaTime) {
            state.points[0].x += delta_time.alpha as f32;
        }
    }

    #[test]
    fn named_evaluator_has_one_associated_output() {
        let mut animation = Iterative::new(VItem::default(), MoveRight);
        Eval::step(
            &mut animation,
            &Time::default(),
            &DeltaTime {
                alpha: 0.25,
                global_secs: 1.0,
            },
        );

        assert_eq!(Eval::sample(&animation, &Time::default()).points[0].x, 0.25);
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
                    |state: &mut VItem, _time: &Time, delta_time: &DeltaTime| {
                        state.points[0].x += delta_time.alpha as f32;
                    },
                )
                .with_duration(2.0),
            );
            scene.seal().into_evaluator(120.0)
        }

        let run = |seek: bool| {
            let mut ev = build();
            let mut trace = Vec::new();
            for sec in [0.5, 1.0, 1.5, 2.0] {
                if seek {
                    ev.seek(sec);
                } else {
                    ev.advance_to(sec);
                }
                let mut frame = Vec::new();
                ev.sample_into(&mut frame);
                let x = frame.iter().find_map(|(_, item)| match item {
                    CoreItem::VItem(v) => Some(v.points[0].x),
                    _ => None,
                });
                trace.push(x);
            }
            trace
        };

        let forward = run(false);
        assert_eq!(forward, run(true));
        // 2s at 120Hz: 240 steps of Δalpha = 1/240 each → x ≈ 1.0
        let x = forward.last().unwrap().unwrap();
        assert!((x - 1.0).abs() < 1e-3, "x = {x}");
    }
}
