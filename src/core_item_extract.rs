//! Extracts built-in core items from evaluated frames into the render world.

use std::{
    collections::{HashMap, HashSet},
    hash::{BuildHasherDefault, Hasher},
};

use bevy_ecs::prelude::*;
use ranim_core::{
    core_item::{CoreItem, camera_frame::CameraFrame, mesh_item::MeshItem, vitem::VItem},
    frame::{EvaluatedFrame, FrameItemId},
};
use ranim_render::{schedule::RenderExtract, world::SceneOrder};

/// Extracts the evaluated frame's [`CoreItem`] batch into the render world.
///
/// Maintains core item entity identities, scene order, changed components, and
/// removal of stale entities. This plugin does not install any GPU pipelines.
/// Install this Bevy plugin through [`ranim_render::Renderer::new`] alongside
/// the required rendering plugins.
pub struct CoreItemExtractPlugin;

impl bevy_app::Plugin for CoreItemExtractPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<CoreItemEntities>()
            .add_systems(RenderExtract, extract_core_items);
    }
}

type FrameItemIdBuildHasher = BuildHasherDefault<FrameItemIdHasher>;
type FrameItemIdMap<V> = HashMap<FrameItemId, V, FrameItemIdBuildHasher>;
type FrameItemIdSet = HashSet<FrameItemId, FrameItemIdBuildHasher>;

/// Fast deterministic hashing for trusted, internal `(animation_id, part)` keys.
struct FrameItemIdHasher(u64);

impl Default for FrameItemIdHasher {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for FrameItemIdHasher {
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

#[derive(Resource, Default)]
struct CoreItemEntities(FrameItemIdMap<Entity>);

fn extract_core_items(world: &mut World) {
    let _span = ranim_render::cpu_probe::span("reconcile");
    world.resource_scope(|world, frame: Mut<EvaluatedFrame>| {
        let items = frame.items::<CoreItem>();
        let mut seen = FrameItemIdSet::with_capacity_and_hasher(
            items.len(),
            FrameItemIdBuildHasher::default(),
        );

        for item in items {
            let id = item.id;
            assert!(seen.insert(id), "duplicate CoreItem id {id:?}");

            let (entity, is_new) =
                if let Some(entity) = world.resource::<CoreItemEntities>().0.get(&id) {
                    (*entity, false)
                } else {
                    let entity = world.spawn(SceneOrder(item.order)).id();
                    world
                        .resource_mut::<CoreItemEntities>()
                        .0
                        .insert(id, entity);
                    (entity, true)
                };

            replace_component(world, entity, &SceneOrder(item.order));
            match &item.value {
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
    });
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
    use bevy_app::SubApp;
    use bevy_ecs::{query::Changed, schedule::ScheduleLabel};

    use super::*;

    fn extraction_app() -> SubApp {
        let mut app = SubApp::new();
        app.update_schedule = Some(RenderExtract.intern());
        app.init_resource::<EvaluatedFrame>()
            .add_plugins(CoreItemExtractPlugin);
        app
    }

    #[test]
    fn extraction_preserves_global_order_across_typed_batches() {
        let mut app = extraction_app();
        let mut frame = EvaluatedFrame::new();
        frame.push((0, 0), CoreItem::CameraFrame(CameraFrame::default()));
        frame.push((1, 0), 10_u32);
        frame.push((2, 0), CoreItem::VItem(VItem::default()));
        frame.push((3, 0), 20_u32);
        frame.push((4, 0), CoreItem::MeshItem(MeshItem::default()));
        app.insert_resource(frame);
        app.update();

        let world = app.world();
        let entities = world.resource::<CoreItemEntities>();
        assert_eq!(entities.0.len(), 3);
        for order in [0, 2, 4] {
            let entity = entities.0[&(order, 0)];
            assert_eq!(world.get::<SceneOrder>(entity), Some(&SceneOrder(order)));
        }
        let frame = world.resource::<EvaluatedFrame>();
        assert_eq!(frame.len(), 5);
        assert_eq!(frame.items::<CoreItem>().len(), 3);
        assert_eq!(frame.items::<u32>()[1].value, 20);
    }

    #[test]
    fn extraction_preserves_identity_skips_unchanged_items_and_replaces_kinds() {
        #[derive(Resource, Default)]
        struct ChangeCounts(Vec<usize>);

        let mut app = extraction_app();
        app.init_resource::<ChangeCounts>().add_systems(
            RenderExtract,
            (|items: Query<Entity, Changed<VItem>>, mut counts: ResMut<ChangeCounts>| {
                counts.0.push(items.iter().count());
            })
            .after(extract_core_items),
        );
        app.world_mut()
            .resource_mut::<EvaluatedFrame>()
            .push((1, 2), CoreItem::VItem(VItem::default()));
        app.update();
        let entity = app.world().resource::<CoreItemEntities>().0[&(1, 2)];

        app.update();
        assert_eq!(
            app.world().resource::<CoreItemEntities>().0[&(1, 2)],
            entity
        );
        assert_eq!(app.world().resource::<ChangeCounts>().0, [1, 0]);

        let mut changed = VItem::default();
        changed.points[0].x = 1.0;
        app.world_mut()
            .resource_mut::<EvaluatedFrame>()
            .update([((1, 2), CoreItem::VItem(changed))]);
        app.update();
        assert_eq!(app.world().resource::<ChangeCounts>().0, [1, 0, 1]);

        app.world_mut()
            .resource_mut::<EvaluatedFrame>()
            .update([((1, 2), CoreItem::MeshItem(MeshItem::default()))]);
        app.update();
        assert_eq!(
            app.world().resource::<CoreItemEntities>().0[&(1, 2)],
            entity
        );
        assert!(app.world().get::<MeshItem>(entity).is_some());
        assert!(app.world().get::<VItem>(entity).is_none());
        assert!(app.world().get::<CameraFrame>(entity).is_none());

        app.world_mut().resource_mut::<EvaluatedFrame>().clear();
        app.update();
        assert!(app.world().resource::<CoreItemEntities>().0.is_empty());
        assert!(!app.world().entities().contains(entity));
    }

    #[test]
    fn missing_core_item_batch_removes_previous_entities() {
        let mut app = extraction_app();
        app.world_mut()
            .resource_mut::<EvaluatedFrame>()
            .push((1, 2), CoreItem::VItem(VItem::default()));
        app.world_mut()
            .resource_mut::<EvaluatedFrame>()
            .push((3, 0), CoreItem::MeshItem(MeshItem::default()));
        app.update();
        let entity = app.world().resource::<CoreItemEntities>().0[&(1, 2)];
        let removed = app.world().resource::<CoreItemEntities>().0[&(3, 0)];

        app.world_mut()
            .resource_mut::<EvaluatedFrame>()
            .update([((1, 2), CoreItem::VItem(VItem::default()))]);
        app.update();
        assert_eq!(
            app.world().resource::<CoreItemEntities>().0[&(1, 2)],
            entity
        );
        assert!(!app.world().entities().contains(removed));

        let mut frame = EvaluatedFrame::new();
        frame.push((1, 2), 7_u32);
        app.insert_resource(frame);
        app.update();

        assert!(app.world().resource::<CoreItemEntities>().0.is_empty());
        assert!(!app.world().entities().contains(entity));
        assert_eq!(
            app.world().resource::<EvaluatedFrame>().items::<u32>()[0].value,
            7
        );
    }

    #[test]
    fn core_item_extraction_requires_its_plugin() {
        let mut app = SubApp::new();
        app.update_schedule = Some(RenderExtract.intern());
        app.init_schedule(RenderExtract);
        let mut frame = EvaluatedFrame::new();
        frame.push((1, 2), CoreItem::VItem(VItem::default()));
        app.insert_resource(frame);
        app.update();

        let world = app.world_mut();
        assert_eq!(world.query::<&VItem>().iter(world).count(), 0);
        assert!(!world.contains_resource::<CoreItemEntities>());
        assert_eq!(
            world.resource::<EvaluatedFrame>().items::<CoreItem>().len(),
            1
        );
    }

    #[test]
    #[should_panic(expected = "duplicate CoreItem id (1, 2)")]
    fn duplicate_core_item_ids_are_rejected() {
        let mut app = extraction_app();
        app.world_mut().resource_mut::<EvaluatedFrame>().update([
            ((1, 2), CoreItem::VItem(VItem::default())),
            ((1, 2), CoreItem::MeshItem(MeshItem::default())),
        ]);
        app.update();
    }
}
