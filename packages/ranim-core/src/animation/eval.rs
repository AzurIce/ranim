//! Evaluation protocols and the standard author-facing adapters.
//!
//! [`Eval`] is the single visual leaf protocol;
//! [`EvalExt`] adds build-time conveniences;
//! [`pure::Pure`] adapts a closure
//! and [`iterative::Iterative`]
//! adapts a stepping function into that protocol. `EvalDyn` is the
//! runtime-erased leaf dispatch held by `LeafContent::Anim`.

use dyn_clone::DynClone;

use crate::frame::EvaluatedFrame;

use super::build::{Paramed, PlaybackExt};

/// Iterative (stateful, stepped) evaluation.
pub mod iterative;
/// Pure (closed-form) evaluation adapters.
pub mod pure;

/// The general animation segment protocol: what the runtime can do with a segment.
///
/// An animation's content is immutable once defined: it is a pure function of
/// its own normalized progress `alpha ∈ [0, 1]`. The protocol exposes a single
/// entry — `eval_alpha` — and it is a **pure query** on `&self` (evaluating
/// the same `alpha` always yields the same `Output`, regardless of call order
/// or repetition). No evaluator sees seconds or the scene clock; the owning
/// cell remaps time to progress before calling in.
///
/// Stateful (iterative) segments memoize their integration behind a snapshot so
/// repeated queries are cheap; pure segments are a closed form. The standard
/// author-facing adapters live here too:
/// [`Iterative`](crate::animation::eval::iterative::Iterative) turns an
/// [`IterativeEval`](crate::animation::eval::iterative::IterativeEval) step
/// function into an `Eval`, and
/// [`Pure`](crate::animation::eval::pure::Pure) wraps a raw closure
/// `Fn(f64) -> T`. Implementing `Eval` directly remains the path for
/// exotic segments.
pub trait Eval {
    /// Value produced by this evaluator.
    ///
    /// Playback requires `Clone + Send + Sync + 'static`. Each sample retains
    /// the complete value, including any containers or placement wrappers.
    type Output;

    /// Evaluate the segment's content at normalized progress `alpha`.
    fn eval_alpha(&self, alpha: f64) -> Self::Output;

    /// The content resolution declared by iterative segments: `1/N` progress
    /// per integration step (`N` declared via
    /// [`Iterative::with_steps`](crate::animation::eval::iterative::Iterative::with_steps)).
    ///
    /// `None` for non-iterative segments. This is an introspection query for
    /// tooling (e.g. `ranim inspect tree`); it does not affect evaluation.
    fn sim_step(&self) -> Option<f64> {
        None
    }
}

/// Build-time conveniences over [`Eval`], split out so `Eval` stays a single
/// primitive (`eval_alpha`).
///
/// The only consumers today are the built-in pure-animation families, which use
/// `apply_to` to write an item's end state (or `apply_alpha_to` to write an
/// arbitrary progress state) while constructing the animation.
pub trait EvalExt: Eval + Sized {
    /// Write the state at progress `alpha` into `item` and return this
    /// evaluator (defined through `eval_alpha`).
    fn apply_alpha_to(self, item: &mut Self::Output, alpha: f64) -> Self {
        *item = self.eval_alpha(alpha);
        self
    }

    /// Write the end state (`alpha == 1.0`) into `item` and return this
    /// evaluator (defined through [`EvalExt::apply_alpha_to`]).
    fn apply_to(self, item: &mut Self::Output) -> Self {
        self.apply_alpha_to(item, 1.0)
    }
}

impl<E: Eval + Sized> EvalExt for E {}

/// Frame insertion state shared by the leaves of one top-level animation.
pub(crate) struct OutputContext<'a> {
    frame: &'a mut EvaluatedFrame,
    source_id: usize,
    start_order: usize,
}

impl<'a> OutputContext<'a> {
    pub(crate) fn new(frame: &'a mut EvaluatedFrame, source_id: usize) -> Self {
        let start_order = frame.len();
        Self {
            frame,
            source_id,
            start_order,
        }
    }

    fn push<T: Send + Sync + 'static>(&mut self, value: T) {
        let part = self.frame.len() - self.start_order;
        self.frame.push((self.source_id, part), value);
    }
}

/// A cloneable complete output captured by a hold.
#[derive(Clone)]
pub(crate) struct DynOutput(Box<dyn CapturedOutput>);

trait CapturedOutput: DynClone + Send + Sync {
    fn write_box(self: Box<Self>, output: &mut OutputContext<'_>);
}

impl<T: Clone + Send + Sync + 'static> CapturedOutput for T {
    fn write_box(self: Box<Self>, output: &mut OutputContext<'_>) {
        output.push(*self);
    }
}

dyn_clone::clone_trait_object!(CapturedOutput);

impl DynOutput {
    pub(super) fn write(self, output: &mut OutputContext<'_>) {
        self.0.write_box(output);
    }
}

/// The erased visual-leaf protocol held by the runtime tree.
///
/// Ordinary sampling inserts each complete typed output directly. Holds capture
/// the same raw values and replay them without introducing a container payload.
pub(super) trait EvalDyn {
    fn eval_frame(&self, alpha: f64, output: &mut OutputContext<'_>);
    fn eval_capture(&self, alpha: f64, output: &mut Vec<DynOutput>);

    /// The iterative content step, if this leaf is an iterative segment.
    fn sim_step(&self) -> Option<f64> {
        None
    }
}

impl<E> EvalDyn for E
where
    E: Eval,
    E::Output: Clone + Send + Sync + 'static,
{
    fn eval_frame(&self, alpha: f64, output: &mut OutputContext<'_>) {
        output.push(self.eval_alpha(alpha));
    }

    fn eval_capture(&self, alpha: f64, output: &mut Vec<DynOutput>) {
        output.push(DynOutput(Box::new(self.eval_alpha(alpha))));
    }

    fn sim_step(&self) -> Option<f64> {
        Eval::sim_step(self)
    }
}

/// A held batch replays each captured value with its original concrete type.
pub(super) struct HeldOutput(Vec<DynOutput>);

impl HeldOutput {
    pub(super) fn new(values: Vec<DynOutput>) -> Self {
        Self(values)
    }
}

impl EvalDyn for HeldOutput {
    fn eval_frame(&self, _alpha: f64, output: &mut OutputContext<'_>) {
        for value in &self.0 {
            value.clone().write(output);
        }
    }

    fn eval_capture(&self, _alpha: f64, output: &mut Vec<DynOutput>) {
        output.extend(self.0.iter().cloned());
    }
}

/// A constant evaluator.
#[derive(Clone)]
pub struct Static<T: Clone>(pub T);

impl<T: Clone> Eval for Static<T> {
    type Output = T;

    fn eval_alpha(&self, _alpha: f64) -> Self::Output {
        self.0.clone()
    }
}

/// Requirement for [`StaticAnim`].
pub trait StaticAnimRequirement: Clone + Send + Sync + 'static {}

impl<T: Clone + Send + Sync + 'static> StaticAnimRequirement for T {}

/// Convenience methods for zero-duration static animations.
pub trait StaticAnim: StaticAnimRequirement + Sized {
    /// Show this value.
    fn show(&self) -> Paramed<Static<Self>>;
    /// Hide this value.
    fn hide(&self) -> Paramed<Static<Self>>;
}

impl<T: StaticAnimRequirement + 'static> StaticAnim for T {
    fn show(&self) -> Paramed<Static<Self>> {
        Static(self.clone()).with_duration(0.0)
    }

    fn hide(&self) -> Paramed<Static<Self>> {
        Static(self.clone()).with_enabled(false).with_duration(0.0)
    }
}
