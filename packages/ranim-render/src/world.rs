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

pub(crate) fn reconcile(world: &mut World, frame: &RenderFrame) {
    let mut seen = CoreItemIdSet::with_capacity_and_hasher(
        frame.items.len(),
        CoreItemIdBuildHasher::default(),
    );

    for (order, (id, item)) in frame.iter().enumerate() {
        assert!(seen.insert(*id), "duplicate CoreItem id {id:?}");

        let (entity, is_new) = if let Some(entity) = world.resource::<CoreItemEntities>().0.get(id)
        {
            (*entity, false)
        } else {
            let entity = world.spawn((CoreItemIdentity(*id), SceneOrder(order))).id();
            world
                .resource_mut::<CoreItemEntities>()
                .0
                .insert(*id, entity);
            (entity, true)
        };

        replace_component(world, entity, &SceneOrder(order));
        match item {
            CoreItem::CameraFrame(item) => {
                let changes_kind = world.get::<CameraFrame>(entity).is_none();
                replace_component(world, entity, item);
                if !is_new && changes_kind {
                    world.entity_mut(entity).remove::<(VItem, MeshItem)>();
                }
            }
            CoreItem::VItem(item) => {
                let changes_kind = world.get::<VItem>(entity).is_none();
                replace_component(world, entity, item);
                if !is_new && changes_kind {
                    world.entity_mut(entity).remove::<(CameraFrame, MeshItem)>();
                }
            }
            CoreItem::MeshItem(item) => {
                let changes_kind = world.get::<MeshItem>(entity).is_none();
                replace_component(world, entity, item);
                if !is_new && changes_kind {
                    world.entity_mut(entity).remove::<(CameraFrame, VItem)>();
                }
            }
        }
    }

    let stale = world
        .resource::<CoreItemEntities>()
        .0
        .keys()
        .filter(|id| !seen.contains(id))
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
