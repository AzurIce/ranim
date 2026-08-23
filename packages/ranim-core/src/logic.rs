//! Logic-side ECS World (M2 stage 1): a retained `LogicWorld` driven by
//! [`ScenePlayer`].
//!
//! Stage 1 establishes the (b)-architecture foundation:
//!
//! - items are **typed bevy components** (`LogicItem` = `Component` + `Extract`);
//! - materialization is **self-describing at the output layer** — every
//!   [`DynItem`](crate::core_item::DynItem) carries a materialize hook
//!   monomorphized at its single erasure point, so the materialize phase is
//!   just the shared `eval` traversal followed by each item upserting itself
//!   by a stable `(animation_id, part)` identity (E4-validated pattern);
//! - extraction is **fused into the upsert**: while the typed value is still
//!   owned, its `CoreItem`s go straight into the entity's retained
//!   [`ExtractedItems`] buffer — one clone per item per frame, same as the
//!   pure path, with no separate extract pass;
//! - [`ScenePlayer`] is the M2 driver: the retained-world counterpart of
//!   [`SceneEvaluator`](crate::SceneEvaluator). Evaluation is a pure query
//!   (`eval_alpha`), so there is no stepping or seek bookkeeping to mirror —
//!   `materialize_at` (upsert with extraction fused in) plus `collect`
//!   (drain) produces an [`EvaluatedFrame`](crate::EvaluatedFrame) identical
//!   to the pure path.
//!
//! The render side is untouched: collect emits `((animation_id, part), CoreItem)`
//! exactly like the pre-ECS path, so `RenderWorld` reconciliation keeps working.

use std::collections::{HashMap, HashSet};

use bevy_ecs::{component::Component, entity::Entity, world::World};

use crate::{
    Extract, SealedRanimScene, TimeMark,
    animation::{AnimationCell, AnimationInfo},
    core_item::{AnyExtractCoreItem, CoreItem},
    scene_evaluator::{EvaluatedFrame, SceneSession},
};

/// A top-level item that can live in the `LogicWorld` as a typed component.
///
/// `LogicItem` = a Send+Sync bevy component that can also degrade to the
/// closed [`CoreItem`] enum. Every item type participating in the world
/// implements this **explicitly** (no blanket): that is what lets the
/// compiler prove `Vec<T>` is *not* a `LogicItem`, so container outputs can
/// take a separate materialization path without a coherence conflict.
pub trait LogicItem: Component + AnyExtractCoreItem {}

// Core render primitives (host-known, always available).
impl LogicItem for crate::core_item::camera_frame::CameraFrame {}
impl LogicItem for crate::core_item::mesh_item::MeshItem {}
impl LogicItem for crate::core_item::vitem::VItem {}

/// A homogeneous batch of items materialized as a single entity holding the
/// whole vector (e.g. `vec![...].show()` or a group animation output).
/// Extraction degrades every element.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Batch<T>(pub Vec<T>);

impl<T: LogicItem + Clone> LogicItem for Batch<T> {}

impl<T: Extract<Target = CoreItem>> Extract for Batch<T> {
    type Target = CoreItem;

    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        self.0.extract_into(buf);
    }
}

/// What an `Eval` leaf may output to be materialized into the `LogicWorld`.
///
/// - single items (`T: LogicItem`) upsert a typed component;
/// - `Vec<T>` (group outputs) upsert a [`Batch<T>`] component holding the
///   whole vector as one entity.
///
/// Every implementation follows the same split (the dylib rule):
///
/// - **host-side** (via the fn pointer in [`MaterializeCtx`]): identity,
///   order, and [`ExtractedItems`] components — these must be registered by
///   the binary that owns the world, because component registration is
///   keyed by `TypeId`, which differs across a dylib boundary;
/// - **hook-side** (wherever the hook was monomorphized): extraction of the
///   owned value and insertion of the typed component `T` — `TypeId`-sensitive
///   operations on `T` are only valid on the side that knows `T`.
pub trait MaterializeOut: AnyExtractCoreItem + Send + Sync + 'static {
    /// Materialize this output at the given slot.
    fn materialize(self, ctx: &mut MaterializeCtx, part: u32);
}

impl<T: LogicItem> MaterializeOut for T {
    fn materialize(self, ctx: &mut MaterializeCtx, part: u32) {
        respawn_on_type_change::<T>(ctx, part);
        let mut extracted = Vec::new();
        self.extract_into(&mut extracted);
        let entity = (ctx.upsert)(ctx, part, extracted);
        ctx.world.entity_mut(entity).insert(self);
    }
}

impl<T: LogicItem + Clone> MaterializeOut for Vec<T> {
    fn materialize(self, ctx: &mut MaterializeCtx, part: u32) {
        respawn_on_type_change::<Batch<T>>(ctx, part);
        let mut extracted = Vec::new();
        self.extract_into(&mut extracted);
        let entity = (ctx.upsert)(ctx, part, extracted);
        ctx.world.entity_mut(entity).insert(Batch(self));
    }
}

/// Respawn the slot's entity when its stored component type differs from the
/// incoming one (e.g. a sequence switched to a child segment whose output
/// type differs), so no stale typed component lingers. Checked hook-side,
/// where the `TypeId` of `T` matches the side that inserted it.
fn respawn_on_type_change<T: Component>(ctx: &mut MaterializeCtx, part: u32) {
    let key = (ctx.animation_id, part);
    if let Some(&entity) = ctx.index.get(&key)
        && !ctx.world.entity(entity).contains::<T>()
    {
        ctx.world.despawn(entity);
        ctx.index.remove(&key);
    }
}

/// Host-known identity component: which logical item this entity is.
///
/// The key is `(animation_id, part)`: `animation_id` is the index of the
/// owning top-level cell, `part` is the stable output slot within it. This
/// matches the identity the pure path used for `RenderFrame`, so the world
/// output is frame-identical.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemIdentity {
    /// Index of the owning top-level cell.
    pub animation_id: u32,
    /// Stable output slot within that cell.
    pub part: u32,
}

/// Host-known draw order. ECS query order is not scene order; the collect
/// phase sorts by this explicitly (D0002 discipline).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SceneOrder {
    /// Index of the owning top-level cell.
    pub animation_id: u32,
    /// Stable output slot within that cell.
    pub part: u32,
}

/// Per-entity extraction output, written at materialize time while the typed
/// value is still owned (stage-2 `Extracted<T>` will generalize this).
///
/// The `Vec` is retained across frames so its allocation is reused: the
/// materialize phase refills it (`clear` + `extract_into`), and the collect
/// phase drains it into the frame buffer (move, not clone).
#[derive(Component, Default)]
pub struct ExtractedItems(pub Vec<CoreItem>);

/// Materialization context threaded down the cell tree.
///
/// This is the internal context of the materialize phase: `part` counts
/// output slots within the current top-level cell; every leaf materialization
/// occupies exactly one slot (multiple extracted `CoreItem`s within a slot
/// are expanded at collect time).
///
/// `upsert` is a fn pointer to host-side code (set by [`ScenePlayer`] when
/// the context is created): the world write for host-known components
/// ([`ItemIdentity`], [`SceneOrder`], [`ExtractedItems`]) must execute in the
/// binary that owns the world, because bevy component registration is keyed
/// by `TypeId`, which differs across a dylib boundary. Hooks monomorphized
/// in a dylib call it indirectly and therefore always upsert host-registered
/// components (the embryonic stage-3 registry shape).
pub struct MaterializeCtx<'w> {
    /// The world being materialized into.
    pub world: &'w mut World,
    /// Cross-frame identity index (`(animation_id, part)` → entity).
    pub index: &'w mut HashMap<(u32, u32), Entity>,
    /// Index of the owning top-level cell.
    pub animation_id: u32,
    /// Next output slot within the current cell.
    pub part: u32,
    /// Keys materialized this frame (drives despawn of stale entities).
    pub seen: &'w mut HashSet<(u32, u32)>,
    /// Host-side slot upsert (see the type-level docs).
    pub upsert: fn(&mut MaterializeCtx<'_>, u32, Vec<CoreItem>) -> Entity,
}

/// Host-side slot upsert: create the entity on first appearance (with
/// host-registered identity/order/extraction components), replace the
/// extraction buffer afterwards, and record the key as seen this frame.
fn upsert_slot(ctx: &mut MaterializeCtx, part: u32, extracted: Vec<CoreItem>) -> Entity {
    let key = (ctx.animation_id, part);
    let entity = match ctx.index.get(&key) {
        Some(&entity) => {
            ctx.world
                .entity_mut(entity)
                .insert(ExtractedItems(extracted));
            entity
        }
        None => ctx
            .world
            .spawn((
                ItemIdentity {
                    animation_id: key.0,
                    part: key.1,
                },
                SceneOrder {
                    animation_id: key.0,
                    part: key.1,
                },
                ExtractedItems(extracted),
            ))
            .id(),
    };
    ctx.index.insert(key, entity);
    ctx.seen.insert(key);
    entity
}

/// The M2 driving session: owns the retained `LogicWorld` and materializes
/// the cell tree into it.
///
/// Evaluation is a pure query (`eval_alpha`), so — like
/// [`SceneEvaluator`](crate::SceneEvaluator) — the session needs no stepping
/// or seek bookkeeping: stateful segments manage direction internally, and
/// `materialize_at` simply asks every active cell to upsert its typed output
/// at the target time.
pub struct ScenePlayer {
    cells: Vec<AnimationCell>,
    world: World,
    /// `(animation_id, part)` → entity, the cross-frame identity index.
    index: HashMap<(u32, u32), Entity>,
    total_secs: f64,
    time_marks: Vec<(f64, TimeMark)>,
    clock: f64,
}

impl ScenePlayer {
    /// Consume a sealed scene and create a driving session with a fresh
    /// `LogicWorld`.
    pub fn new(scene: SealedRanimScene) -> Self {
        Self {
            cells: scene.animations,
            world: World::new(),
            index: HashMap::new(),
            total_secs: scene.total_secs,
            time_marks: scene.time_marks,
            clock: 0.0,
        }
    }

    /// Total scene duration.
    pub fn total_secs(&self) -> f64 {
        self.total_secs
    }

    /// Last materialized target time.
    pub fn clock(&self) -> f64 {
        self.clock
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

    /// Number of live item entities in the `LogicWorld`.
    pub fn item_count(&self) -> usize {
        self.index.len()
    }

    /// Take the `LogicWorld` out for render-side direct extraction (world
    /// exchange): the render side reconciles from it and hands it back via
    /// [`put_world`](Self::put_world) before the next `materialize_at`.
    pub fn take_world(&mut self) -> World {
        std::mem::replace(&mut self.world, World::new())
    }

    /// Put the `LogicWorld` back after render-side direct extraction.
    pub fn put_world(&mut self, world: World) {
        self.world = world;
    }

    /// Materialize the scene at `render_secs` into the `LogicWorld`: every
    /// active cell evaluates (the same pure query as the render path) and
    /// each resulting [`DynItem`](crate::core_item::DynItem) upserts itself
    /// as a typed component via its captured materialize hook; entities whose
    /// identity no longer appears are despawned ("entity lifetime = producer
    /// lifetime").
    pub fn materialize_at(&mut self, render_secs: f64) {
        let mut seen = HashSet::new();
        for (animation_id, cell) in self.cells.iter().enumerate() {
            let mut items = Vec::new();
            cell.eval_at(render_secs, &mut items);
            if items.is_empty() {
                continue;
            }
            let mut ctx = MaterializeCtx {
                world: &mut self.world,
                index: &mut self.index,
                animation_id: animation_id as u32,
                part: 0,
                seen: &mut seen,
                upsert: upsert_slot,
            };
            for item in items {
                let part = ctx.part;
                ctx.part += 1;
                item.materialize(&mut ctx, part);
            }
        }
        let stale: Vec<(u32, u32)> = self
            .index
            .keys()
            .filter(|key| !seen.contains(key))
            .copied()
            .collect();
        for key in stale {
            if let Some(entity) = self.index.remove(&key) {
                self.world.despawn(entity);
            }
        }
        self.clock = render_secs;
    }

    /// Collect the extracted items into an [`EvaluatedFrame`](crate::EvaluatedFrame)
    /// sorted by scene order, **draining** each entity's [`ExtractedItems`]
    /// buffer (the elements are moved into `out`, the allocation stays for
    /// the next frame). The frame identity `(animation_id, part)` uses the
    /// same flattened part indexing as the pure path, so output is
    /// frame-identical and the renderer needs no changes.
    pub fn collect(&mut self, out: &mut EvaluatedFrame) {
        let mut ordered: Vec<(SceneOrder, Entity)> = Vec::new();
        for eref in self.world.iter_entities() {
            let Some(order) = eref.get::<SceneOrder>() else {
                continue;
            };
            if eref.get::<ItemIdentity>().is_none() {
                continue;
            }
            ordered.push((*order, eref.id()));
        }
        ordered.sort_by_key(|(order, _)| *order);

        let mut current_animation: Option<u32> = None;
        let mut flat_part = 0usize;
        for (order, entity) in ordered {
            if current_animation != Some(order.animation_id) {
                current_animation = Some(order.animation_id);
                flat_part = 0;
            }
            let mut entity_mut = self.world.entity_mut(entity);
            let Some(mut extracted) = entity_mut.get_mut::<ExtractedItems>() else {
                continue;
            };
            let animation_id = order.animation_id as usize;
            let len = extracted.0.len();
            out.extend(
                extracted
                    .0
                    .drain(..)
                    .enumerate()
                    .map(|(i, core)| ((animation_id, flat_part + i), core)),
            );
            flat_part += len;
        }
    }

    /// One frame: materialize at `render_secs` (upsert + extract fused), then
    /// collect into `out`. Convenience wrapper mirroring the pure path's
    /// [`SceneEvaluator::sample_at`](crate::SceneEvaluator::sample_at).
    pub fn frame(&mut self, render_secs: f64, out: &mut EvaluatedFrame) {
        self.materialize_at(render_secs);
        self.collect(out);
    }
}

impl SceneSession for ScenePlayer {
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
        self.frame(render_secs, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RanimScene, SealedRanimScene, seq,
        animation::{AnimationExt, StaticAnim, eval::Eval, eval::iterative::Iterative},
        core_item::{camera_frame::CameraFrame, mesh_item::MeshItem, vitem::VItem},
    };

    /// Pure evaluator: a vitem sliding along X with `alpha`.
    struct MoveX;
    impl Eval for MoveX {
        type Output = VItem;

        fn eval_alpha(&self, alpha: f64) -> Self::Output {
            let mut v = VItem::default();
            v.points[0].x = alpha as f32;
            v
        }
    }

    /// Iterative segment: x accumulates `rate * delta_alpha` per step.
    fn drift(rate: f32) -> impl crate::animation::Placeable {
        Iterative::from_fn(
            VItem::default(),
            move |state: &mut VItem, _alpha: f64, delta_alpha: f64| {
                state.points[0].x += rate * delta_alpha as f32;
            },
        )
        .with_duration(1.0)
    }

    fn build() -> SealedRanimScene {
        let mut scene = RanimScene::new();
        scene.play(CameraFrame::default().show().with_duration(3.0));
        scene.play(MoveX.with_duration(1.0));
        scene.play(seq![drift(1.0), drift(2.0)]);
        scene.seal()
    }

    /// The world path must produce frame-identical output to the pure path.
    #[test]
    fn player_output_matches_evaluator() {
        let mut ev = build().into_evaluator();
        let mut pl = ScenePlayer::new(build());

        for t in [0.0, 0.3, 0.7, 1.0, 1.2, 1.8, 2.5, 3.0] {
            let mut frame_ev = Vec::new();
            ev.sample_at(t, &mut frame_ev);

            let mut frame_pl = Vec::new();
            pl.frame(t, &mut frame_pl);

            assert_eq!(frame_pl, frame_ev, "frame mismatch at t = {t}");
        }
    }

    /// A backward jump forces iterative leaves to reset+replay internally;
    /// sampling the same target again must match the forward result.
    #[test]
    fn player_seek_matches_forward() {
        let mut pl = ScenePlayer::new(build());
        let mut forward = Vec::new();
        pl.frame(1.5, &mut forward);

        pl.frame(0.4, &mut Vec::new()); // backward jump
        let mut replayed = Vec::new();
        pl.frame(1.5, &mut replayed);

        assert_eq!(replayed, forward, "seek replay must equal forward sampling");
    }

    /// Entities are retained across frames and updated by identity, not
    /// re-spawned; stale entities are despawned when their producer leaves.
    #[test]
    fn entities_are_retained_and_lifetime_follows_producer() {
        let mut pl = ScenePlayer::new(build());

        // t=0.3: camera + movex + seq[first drift] active → 3 entities.
        pl.frame(0.3, &mut Vec::new());
        assert_eq!(pl.item_count(), 3);

        // Same time again: identity upsert, no new entities.
        pl.frame(0.3, &mut Vec::new());
        assert_eq!(pl.item_count(), 3);

        // t=1.5: movex ended (despawned), seq moved to second drift (same
        // entity slot, upserted) → camera + seq = 2.
        pl.frame(1.5, &mut Vec::new());
        assert_eq!(pl.item_count(), 2);

        // t=2.5: drift seq ended → 1 entity (camera only).
        pl.frame(2.5, &mut Vec::new());
        assert_eq!(pl.item_count(), 1);

        // Backwards jump: entities whose producers are active again respawn.
        pl.materialize_at(0.3);
        assert_eq!(pl.item_count(), 3);
    }

    /// A slot reused across sequence segments with different output types
    /// respawns its entity so the extractor matches the stored component.
    #[test]
    fn slot_type_change_respawns_entity() {
        struct ShowMesh;
        impl Eval for ShowMesh {
            type Output = MeshItem;

            fn eval_alpha(&self, _alpha: f64) -> Self::Output {
                MeshItem::default()
            }
        }

        let mut scene = RanimScene::new();
        scene.play(seq![MoveX.with_duration(1.0), ShowMesh.with_duration(1.0)]);
        let mut pl = ScenePlayer::new(scene.seal());

        let mut frame = Vec::new();
        pl.frame(0.5, &mut frame);
        assert_eq!(frame.len(), 1);
        assert!(matches!(frame[0].1, CoreItem::VItem(_)));

        let mut frame = Vec::new();
        pl.frame(1.5, &mut frame);
        assert_eq!(frame.len(), 1);
        assert!(matches!(frame[0].1, CoreItem::MeshItem(_)));
    }

    /// A held static snapshot is type-erased at capture time; the DynItem
    /// hook must still materialize it correctly (frame-identical to the
    /// pure path).
    #[test]
    fn held_static_snapshot_materializes() {
        use crate::animation::sequence::AnimSequence;

        fn build_hold() -> SealedRanimScene {
            let mut scene = RanimScene::new();
            let mut s = AnimSequence::new();
            s.push(MoveX.with_duration(1.0)).hold(1.0);
            scene.play(s);
            scene.seal()
        }

        let mut ev = build_hold().into_evaluator();
        let mut pl = ScenePlayer::new(build_hold());
        for t in [0.5, 1.0, 1.5, 2.0] {
            let mut frame_ev = Vec::new();
            ev.sample_at(t, &mut frame_ev);
            let mut frame_pl = Vec::new();
            pl.frame(t, &mut frame_pl);
            assert_eq!(frame_pl, frame_ev, "frame mismatch at t = {t}");
        }
    }
}
