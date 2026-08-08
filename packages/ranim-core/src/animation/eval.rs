//! Evaluation protocols and the standard author-facing adapters.
//!
//! [`Eval`] is the single leaf protocol;
//! [`EvalExt`](crate::animation::eval::EvalExt) adds build-time conveniences;
//! [`pure::Pure`](crate::animation::eval::pure::Pure) adapts a closure
//! and [`iterative::Iterative`](crate::animation::eval::iterative::Iterative)
//! adapts a stepping function into that protocol. [`EvalDyn`] is the
//! runtime-erased counterpart used by [`AnimationCell`].
//!
//! [`AnimationCell`]: super::AnimationCell

use crate::{
    core_item::DynItem,
    logic::{MaterializeCtx, MaterializeOut, upsert_item},
};

use super::{AnimationInfo, AnimationInfoKind};

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

/// An auto implemented trait for erasing `Eval<Output = T>` where T: MaterializeOut
pub(super) trait EvalDyn {
    /// Evaluate this node's content at its own normalized progress `alpha`,
    /// pushing the resulting erased items into `output`.
    ///
    /// Leaves evaluate/project at `alpha`; containers remap `alpha` into their
    /// content coordinates and recurse into their active children.
    fn eval_dyn(&self, _alpha: f64, _output: &mut Vec<DynItem>) {}

    /// Materialize this node's typed output into the `LogicWorld` (M2 stage 1).
    ///
    /// The typed twin of [`eval_dyn`](EvalDyn::eval_dyn): leaves evaluate at
    /// `alpha` and upsert their `Output` as a typed component at the current
    /// identity slot; containers forward to their active children. Default
    /// no-op for nodes without a typed output.
    fn materialize_dyn(&self, _alpha: f64, _ctx: &mut MaterializeCtx) {}

    fn info_kind(&self) -> AnimationInfoKind {
        AnimationInfoKind::Eval
    }

    fn content_duration_secs(&self) -> f64 {
        1.0
    }

    fn child_infos(&self) -> Vec<AnimationInfo> {
        Vec::new()
    }

    /// The iterative content step, if this node is an iterative segment.
    fn sim_step(&self) -> Option<f64> {
        None
    }
}

pub(super) struct StaticDynItems(pub(super) Vec<DynItem>);

impl EvalDyn for StaticDynItems {
    fn eval_dyn(&self, _alpha: f64, output: &mut Vec<DynItem>) {
        output.extend(self.0.iter().cloned());
    }

    fn materialize_dyn(&self, _alpha: f64, ctx: &mut MaterializeCtx) {
        // Best effort for the core item family (already `LogicItem`); other
        // static outputs are skipped until they become components (TODO M2).
        use crate::core_item::{camera_frame::CameraFrame, mesh_item::MeshItem, vitem::VItem};
        use std::any::Any;
        for item in &self.0 {
            let any: &dyn Any = item.0.as_ref();
            let part = ctx.part;
            if let Some(v) = any.downcast_ref::<VItem>() {
                ctx.part += 1;
                upsert_item(ctx, part, v.clone());
            } else if let Some(m) = any.downcast_ref::<MeshItem>() {
                ctx.part += 1;
                upsert_item(ctx, part, m.clone());
            } else if let Some(c) = any.downcast_ref::<CameraFrame>() {
                ctx.part += 1;
                upsert_item(ctx, part, c.clone());
            }
        }
    }

    fn info_kind(&self) -> AnimationInfoKind {
        AnimationInfoKind::Static
    }

    fn content_duration_secs(&self) -> f64 {
        0.0
    }
}

impl<E> EvalDyn for E
where
    E: Eval,
    E::Output: MaterializeOut,
{
    fn eval_dyn(&self, alpha: f64, output: &mut Vec<DynItem>) {
        output.push(DynItem(Box::new(self.eval_alpha(alpha))));
    }

    fn materialize_dyn(&self, alpha: f64, ctx: &mut MaterializeCtx) {
        let part = ctx.part;
        ctx.part += 1;
        self.eval_alpha(alpha).materialize(ctx, part);
    }

    fn sim_step(&self) -> Option<f64> {
        Eval::sim_step(self)
    }
}
