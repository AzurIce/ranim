use std::{
    collections::{HashMap, HashSet},
    hash::{BuildHasherDefault, Hasher},
};

use bevy_ecs::prelude::*;
use ranim_core::core_item::{
    CoreItem, camera_frame::CameraFrame, mesh_item::MeshItem, vitem::VItem,
};

pub type CoreItemId = (usize, usize);

type CoreItemIdBuildHasher = BuildHasherDefault<CoreItemIdHasher>;
type CoreItemIdMap<V> = HashMap<CoreItemId, V, CoreItemIdBuildHasher>;
type CoreItemIdSet = HashSet<CoreItemId, CoreItemIdBuildHasher>;

/// Fast deterministic hashing for trusted, internal `(animation_id, part)` keys.
struct CoreItemIdHasher(u64);

impl Default for CoreItemIdHasher {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for CoreItemIdHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn write_usize(&mut self, value: usize) {
        self.0 ^= value as u64;
        self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

/// A reusable, frame-local transport buffer between evaluation and rendering.
#[derive(Default)]
pub struct RenderFrame {
    items: Vec<(CoreItemId, CoreItem)>,
}

impl RenderFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, items: impl Iterator<Item = (CoreItemId, CoreItem)>) {
        self.items.clear();
        self.items.extend(items);
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &(CoreItemId, CoreItem)> {
        self.items.iter()
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CoreItemIdentity(pub CoreItemId);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SceneOrder(pub usize);

#[derive(Resource, Default)]
pub(crate) struct CoreItemEntities(CoreItemIdMap<Entity>);

/// Get or spawn the render entity for `id`, refreshing its draw order.
fn upsert_entity(world: &mut World, id: CoreItemId, order: usize) -> (Entity, bool) {
    if let Some(entity) = world.resource::<CoreItemEntities>().0.get(&id) {
        let entity = *entity;
        replace_component(world, entity, &SceneOrder(order));
        (entity, false)
    } else {
        let entity = world.spawn((CoreItemIdentity(id), SceneOrder(order))).id();
        world
            .resource_mut::<CoreItemEntities>()
            .0
            .insert(id, entity);
        (entity, true)
    }
}

/// Despawn render entities whose identity did not appear this frame.
fn despawn_stale(world: &mut World, seen: &CoreItemIdSet) {
    let stale = world
        .resource::<CoreItemEntities>()
        .0
        .keys()
        .filter(|id| !seen.contains(*id))
        .copied()
        .collect::<Vec<_>>();
    for id in stale {
        let entity = world
            .resource_mut::<CoreItemEntities>()
            .0
            .remove(&id)
            .unwrap();
        world.despawn(entity);
    }
}

/// Replace a component with an owned value only when it actually changed
/// (preserving change ticks), moving instead of cloning.
fn replace_component_owned<T>(world: &mut World, entity: Entity, value: T)
where
    T: Component + PartialEq,
{
    if world.get::<T>(entity) != Some(&value) {
        world.entity_mut(entity).insert(value);
    }
}

/// Upsert one core item at `id`, replacing the previous item kind if needed.
fn upsert_core_item(world: &mut World, id: CoreItemId, order: usize, item: CoreItem) {
    let (entity, is_new) = upsert_entity(world, id, order);
    match item {
        CoreItem::CameraFrame(item) => {
            let changes_kind = world.get::<CameraFrame>(entity).is_none();
            replace_component_owned(world, entity, item);
            if !is_new && changes_kind {
                world.entity_mut(entity).remove::<(VItem, MeshItem)>();
            }
        }
        CoreItem::VItem(item) => {
            let changes_kind = world.get::<VItem>(entity).is_none();
            replace_component_owned(world, entity, item);
            if !is_new && changes_kind {
                world.entity_mut(entity).remove::<(CameraFrame, MeshItem)>();
            }
        }
        CoreItem::MeshItem(item) => {
            let changes_kind = world.get::<MeshItem>(entity).is_none();
            replace_component_owned(world, entity, item);
            if !is_new && changes_kind {
                world.entity_mut(entity).remove::<(CameraFrame, VItem)>();
            }
        }
    }
}

/// Reconcile the render world from a transport frame.
pub fn reconcile(world: &mut World, frame: &RenderFrame) {
    world.init_resource::<CoreItemEntities>();
    let mut seen = CoreItemIdSet::with_capacity_and_hasher(
        frame.items.len(),
        CoreItemIdBuildHasher::default(),
    );

    for (order, (id, item)) in frame.iter().enumerate() {
        assert!(seen.insert(*id), "duplicate CoreItem id {id:?}");
        upsert_core_item(world, *id, order, item.clone());
    }

    despawn_stale(world, &seen);
}

/// Reconcile the render world directly from a `LogicWorld` (M2 direct
/// extraction): drains each logic entity's
/// [`ExtractedItems`](ranim_core::logic::ExtractedItems) and upserts the
/// render components with the same identity/order mapping as
/// [`reconcile`], moving values instead of cloning them.
pub fn reconcile_logic(world: &mut World, logic: &mut World) {
    use ranim_core::logic::{ExtractedItems, SceneOrder as LogicSceneOrder};

    world.init_resource::<CoreItemEntities>();

    let mut ordered: Vec<(u32, u32, Entity)> = Vec::new();
    for eref in logic.iter_entities() {
        let Some(order) = eref.get::<LogicSceneOrder>() else {
            continue;
        };
        ordered.push((order.animation_id, order.part, eref.id()));
    }
    ordered.sort();

    let mut seen = CoreItemIdSet::with_capacity_and_hasher(
        ordered.len(),
        CoreItemIdBuildHasher::default(),
    );
    let mut order = 0usize;
    let mut current_animation = None;
    let mut flat_part = 0usize;
    for (animation_id, _slot, logic_entity) in ordered {
        if current_animation != Some(animation_id) {
            current_animation = Some(animation_id);
            flat_part = 0;
        }
        let mut entity_mut = logic.entity_mut(logic_entity);
        let mut extracted = entity_mut.get_mut::<ExtractedItems>().unwrap();
        for item in extracted.0.drain(..) {
            let id = (animation_id as usize, flat_part);
            flat_part += 1;
            assert!(seen.insert(id), "duplicate CoreItem id {id:?}");
            upsert_core_item(world, id, order, item);
            order += 1;
        }
    }

    despawn_stale(world, &seen);
}

fn replace_component<T>(world: &mut World, entity: Entity, value: &T)
where
    T: Component + PartialEq + Clone,
{
    if world.get::<T>(entity) != Some(value) {
        world.entity_mut(entity).insert(value.clone());
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::query::Changed;

    use super::*;

    #[test]
    fn reconciliation_preserves_identity_and_change_ticks() {
        let mut world = World::new();
        world.init_resource::<CoreItemEntities>();
        let mut frame = RenderFrame::new();
        frame.update([((1, 2), CoreItem::VItem(VItem::default()))].into_iter());
        reconcile(&mut world, &frame);
        let entity = world.resource::<CoreItemEntities>().0[&(1, 2)];
        world.clear_trackers();

        reconcile(&mut world, &frame);
        let mut changed = world.query_filtered::<Entity, Changed<VItem>>();

        assert_eq!(world.resource::<CoreItemEntities>().0[&(1, 2)], entity);
        assert_eq!(changed.iter(&world).count(), 0);
    }

    #[test]
    fn reconciliation_replaces_the_core_item_kind() {
        let mut world = World::new();
        world.init_resource::<CoreItemEntities>();
        let mut frame = RenderFrame::new();
        frame.update([((1, 2), CoreItem::VItem(VItem::default()))].into_iter());
        reconcile(&mut world, &frame);
        let entity = world.resource::<CoreItemEntities>().0[&(1, 2)];

        frame.update([((1, 2), CoreItem::MeshItem(MeshItem::default()))].into_iter());
        reconcile(&mut world, &frame);

        assert_eq!(world.resource::<CoreItemEntities>().0[&(1, 2)], entity);
        assert!(world.get::<MeshItem>(entity).is_some());
        assert!(world.get::<VItem>(entity).is_none());
        assert!(world.get::<CameraFrame>(entity).is_none());
    }
}

#[cfg(test)]
mod logic_tests {
    use ranim_core::{
        RanimScene, ScenePlayer,
        animation::{AnimationExt, StaticAnim},
        core_item::{camera_frame::CameraFrame, vitem::VItem},
    };

    use super::*;

    #[test]
    fn reconcile_logic_moves_camera_and_vitem_into_render_world() {
        let mut scene = RanimScene::new();
        scene.play(CameraFrame::default().show().with_duration(1.0));
        scene.play(VItem::default().show().with_duration(1.0));
        let mut player = ScenePlayer::new(scene.seal());

        player.materialize_at(0.5);
        let mut logic = player.take_world();
        let mut world = World::new();
        reconcile_logic(&mut world, &mut logic);
        player.put_world(logic);

        assert_eq!(
            world.query::<&CameraFrame>().iter(&world).count(),
            1,
            "camera must reach the render world"
        );
        assert_eq!(world.query::<&VItem>().iter(&world).count(), 1);
    }
}
