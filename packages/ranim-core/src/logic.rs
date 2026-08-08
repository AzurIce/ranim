//! Logic-side ECS World (M2 stage 1): a retained `LogicWorld` driven by
//! [`ScenePlayer`].
//!
//! Stage 1 establishes the (b)-architecture foundation:
//!
//! - items are **typed bevy components** (`LogicItem` = `Component` + `Extract`);
//! - materialization happens at the **typed layer** — each `Eval` leaf samples
//!   its `Output` and upserts it into the host-owned `World` by a stable
//!   `(animation_id, part)` identity (E4-validated pattern);
//! - the per-entity [`ItemExtractor`] fn pointer is written by the typed
//!   materializer, so the driver can extract every item without naming its
//!   type (no global registry needed in-process);
//! - [`ScenePlayer`] is the M2 driver: the retained-world counterpart of
//!   [`SceneEvaluator`](crate::SceneEvaluator). Evaluation is a pure query
//!   (`eval_alpha`), so there is no stepping or seek bookkeeping to mirror —
//!   `materialize_at → extract → collect` produces a
//!   [`EvaluatedFrame`](crate::EvaluatedFrame) identical to the pure path.
//!
//! The render side is untouched: collect emits `((animation_id, part), CoreItem)`
//! exactly like the pre-ECS path, so `RenderWorld` reconciliation keeps working.

use std::collections::{HashMap, HashSet};

use bevy_ecs::{component::Component, entity::Entity, world::World};

use crate::{
    Extract, SealedRanimScene,
    animation::AnimationCell,
    core_item::{AnyExtractCoreItem, CoreItem},
    scene_evaluator::EvaluatedFrame,
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
pub trait MaterializeOut: AnyExtractCoreItem + Send + Sync + 'static {
    /// Materialize this output at the given slot.
    fn materialize(self, ctx: &mut MaterializeCtx, part: u32);
}

impl<T: LogicItem> MaterializeOut for T {
    fn materialize(self, ctx: &mut MaterializeCtx, part: u32) {
        upsert_item(ctx, part, self);
    }
}

impl<T: LogicItem + Clone> MaterializeOut for Vec<T> {
    fn materialize(self, ctx: &mut MaterializeCtx, part: u32) {
        upsert_item(ctx, part, Batch(self));
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

/// Type-erased per-entity extractor, written by the typed materializer.
///
/// The driver calls this to degrade the item component to `CoreItem`s without
/// naming its type. In-process this is a plain fn pointer; the dylib path
/// (stage 3) uses the same shape through a global registry instead.
#[derive(Component, Clone, Copy)]
pub struct ItemExtractor(pub fn(Entity, &World, &mut Vec<CoreItem>));

/// Per-entity extraction output. The `Vec` is retained across frames so its
/// allocation is reused (stage-2 `Extracted<T>` will generalize this).
#[derive(Component, Default)]
pub struct ExtractedItems(pub Vec<CoreItem>);

/// Build the type-erased extractor for `T` (monomorphized at the typed site).
fn extractor_of<T: LogicItem>() -> fn(Entity, &World, &mut Vec<CoreItem>) {
    |entity, world, out| {
        if let Some(item) = world.entity(entity).get::<T>() {
            item.extract_into(out);
        }
    }
}

/// Materialization context threaded down the cell tree.
///
/// This is the internal context of the materialize phase: `part` counts
/// output slots within the current top-level cell; every leaf materialization
/// occupies exactly one slot (multiple extracted `CoreItem`s within a slot
/// are expanded at collect time).
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
}

/// Upsert a typed item component at the given `part` slot of the current
/// `animation_id`: create the entity on first appearance, replace the
/// component afterwards, and record the key as seen this frame.
///
/// If the slot already exists but holds a *different* component type (e.g. a
/// sequence switched to a child segment whose output type differs), the stale
/// entity is despawned and respawned so its [`ItemExtractor`] always matches
/// the component actually stored.
pub(crate) fn upsert_item<T: LogicItem>(ctx: &mut MaterializeCtx, part: u32, value: T) {
    let key = (ctx.animation_id, part);
    let entity = match ctx.index.get(&key) {
        Some(&entity) if ctx.world.entity(entity).contains::<T>() => {
            ctx.world.entity_mut(entity).insert(value);
            entity
        }
        existing => {
            if let Some(&entity) = existing {
                ctx.world.despawn(entity);
            }
            ctx.world
                .spawn((
                    ItemIdentity {
                        animation_id: key.0,
                        part: key.1,
                    },
                    SceneOrder {
                        animation_id: key.0,
                        part: key.1,
                    },
                    ItemExtractor(extractor_of::<T>()),
                    ExtractedItems::default(),
                    value,
                ))
                .id()
        }
    };
    ctx.index.insert(key, entity);
    ctx.seen.insert(key);
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

    /// Number of live item entities in the `LogicWorld`.
    pub fn item_count(&self) -> usize {
        self.index.len()
    }

    /// Materialize the scene at `render_secs` into the `LogicWorld`: every
    /// active cell evaluates and upserts its typed item components by
    /// identity; entities whose identity no longer appears are despawned
    /// ("entity lifetime = producer lifetime").
    pub fn materialize_at(&mut self, render_secs: f64) {
        let mut seen = HashSet::new();
        for (animation_id, cell) in self.cells.iter().enumerate() {
            if !cell.active_at(render_secs) {
                continue;
            }
            let mut ctx = MaterializeCtx {
                world: &mut self.world,
                index: &mut self.index,
                animation_id: animation_id as u32,
                part: 0,
                seen: &mut seen,
            };
            cell.materialize_at(render_secs, &mut ctx);
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

    /// Extract every live item to `CoreItem`s via its per-entity extractor.
    pub fn extract(&mut self) {
        let entities: Vec<Entity> = self
            .world
            .iter_entities()
            .filter(|eref| eref.get::<ItemExtractor>().is_some())
            .map(|eref| eref.id())
            .collect();
        for entity in entities {
            let extractor = self.world.entity(entity).get::<ItemExtractor>().unwrap().0;
            let mut buf = Vec::new();
            extractor(entity, &self.world, &mut buf);
            if let Some(mut extracted) = self.world.entity_mut(entity).get_mut::<ExtractedItems>() {
                extracted.0 = buf;
            }
        }
    }

    /// Collect the extracted items into an [`EvaluatedFrame`](crate::EvaluatedFrame)
    /// sorted by scene order. The frame identity `(animation_id, part)` uses
    /// the same flattened part indexing as the pure path, so output is
    /// frame-identical and the renderer needs no changes.
    pub fn collect(&self, out: &mut EvaluatedFrame) {
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
            let Some(extracted) = self.world.entity(entity).get::<ExtractedItems>() else {
                continue;
            };
            for core in &extracted.0 {
                out.push(((order.animation_id as usize, flat_part), core.clone()));
                flat_part += 1;
            }
        }
    }

    /// One frame: materialize at `render_secs`, then extract → collect into
    /// `out`. Convenience wrapper mirroring the pure path's
    /// [`SceneEvaluator::sample_at`](crate::SceneEvaluator::sample_at).
    pub fn frame(&mut self, render_secs: f64, out: &mut EvaluatedFrame) {
        self.materialize_at(render_secs);
        self.extract();
        self.collect(out);
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
        let mut ev = build().into_evaluator(120.0);
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
}
