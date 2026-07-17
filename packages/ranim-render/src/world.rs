//! Renderer-owned ECS world and main-world extraction.

use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    mem,
};

use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    entity::Entity,
    prelude::{Commands, Local, Res, ResMut, Resource},
    query::{QueryFilter, QueryState, ReadOnlyQueryData},
    schedule::{Schedule, ScheduleLabel},
    system::SystemParam,
    world::{World, WorldId},
};
use ranim_core::{
    core_item::{camera_frame::CameraFrame, mesh_item::MeshItem, vitem::VItem},
    store::CoreItemStore,
};

pub use ranim_core::extract::{ExtractComponent, ExtractMany, ExtractOutput};

/// Schedule that extracts main-world data into the render world.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExtractSchedule;

/// The simulation world, available as a render-world resource during extraction.
#[derive(Resource)]
pub struct MainWorld(World);

impl MainWorld {
    /// Borrow the simulation ECS world.
    pub fn world(&self) -> &World {
        &self.0
    }
}

#[derive(Resource, Default)]
struct ScratchMainWorld(World);

#[derive(Resource, Default)]
struct MainSceneOrder {
    orders: HashMap<Entity, usize>,
}

impl MainSceneOrder {
    fn from_store(store: &CoreItemStore) -> Self {
        Self {
            orders: store
                .scene_entities()
                .enumerate()
                .map(|(order, entity)| (entity, order))
                .collect(),
        }
    }

    fn get(&self, entity: Entity) -> Option<usize> {
        self.orders.get(&entity).copied()
    }
}

/// The main-world entity represented by a render root.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MainEntity(pub Entity);

/// The stable order of a render root in the current scene.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderRootOrder(pub usize);

/// The identity of one extracted render-item occurrence.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderItemKey {
    /// The stable render root that owns this item.
    pub root: Entity,
    /// The registered extractor that emitted this item.
    pub extractor: usize,
    /// The part key emitted by the extractor.
    pub part: usize,
}

/// The render root that produced a render-item entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExtractedFrom(pub Entity);

/// The order of an item in the extracted render world.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderItemOrder(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct FrameItemOrder {
    root: usize,
    extractor: usize,
    output: usize,
}

/// How a stale render item is cleaned up.
#[derive(Clone, Copy)]
enum OnStale {
    /// Despawn the primitive entity (`ExtractMany` parts).
    Despawn,
    /// Remove the extracted bundle from the render root entity (1:1
    /// `ExtractComponent` outputs), keeping the root alive for other
    /// extractors that still match the main entity.
    RemoveComponents(fn(&mut World, Entity)),
}

struct ItemSlot {
    entity: Entity,
    on_stale: OnStale,
}

#[derive(Resource, Default)]
struct RenderEntities {
    roots: HashMap<Entity, Entity>,
    items: HashMap<RenderItemKey, ItemSlot>,
    ordered_items: Vec<Entity>,
    seen_roots: HashSet<Entity>,
    seen_items: HashSet<RenderItemKey>,
    frame_items: Vec<(FrameItemOrder, Entity)>,
}

impl RenderEntities {
    fn begin_frame(&mut self) {
        self.seen_roots.clear();
        self.seen_items.clear();
        self.frame_items.clear();
    }

    fn ensure_root(&mut self, source: Entity, order: usize, commands: &mut Commands) -> Entity {
        self.seen_roots.insert(source);
        if let Some(&root) = self.roots.get(&source) {
            commands.entity(root).insert(RenderRootOrder(order));
            root
        } else {
            let root = commands
                .spawn((MainEntity(source), RenderRootOrder(order)))
                .id();
            self.roots.insert(source, root);
            root
        }
    }

    /// Upsert a primitive part entity emitted by an `ExtractMany` extractor.
    fn upsert<B: Bundle>(
        &mut self,
        key: RenderItemKey,
        root_order: usize,
        output_order: usize,
        bundle: B,
        commands: &mut Commands,
    ) {
        assert!(
            self.seen_items.insert(key),
            "extractor {} emitted duplicate part key {} for render root {:?}",
            key.extractor,
            key.part,
            key.root,
        );

        let entity = if let Some(slot) = self.items.get(&key) {
            commands.entity(slot.entity).insert(bundle);
            slot.entity
        } else {
            let entity = commands.spawn((key, ExtractedFrom(key.root), bundle)).id();
            self.items.insert(
                key,
                ItemSlot {
                    entity,
                    on_stale: OnStale::Despawn,
                },
            );
            entity
        };
        self.frame_items.push((
            FrameItemOrder {
                root: root_order,
                extractor: key.extractor,
                output: output_order,
            },
            entity,
        ));
    }

    /// Write a 1:1 `ExtractComponent` output directly onto the render root.
    fn upsert_root<B: Bundle>(
        &mut self,
        key: RenderItemKey,
        root_order: usize,
        bundle: B,
        remove_components: fn(&mut World, Entity),
        commands: &mut Commands,
    ) {
        assert!(
            self.seen_items.insert(key),
            "extractor {} emitted duplicate output for render root {:?}",
            key.extractor,
            key.root,
        );

        commands.entity(key.root).insert(bundle);
        self.items.insert(
            key,
            ItemSlot {
                entity: key.root,
                on_stale: OnStale::RemoveComponents(remove_components),
            },
        );
        self.frame_items.push((
            FrameItemOrder {
                root: root_order,
                extractor: key.extractor,
                output: 0,
            },
            key.root,
        ));
    }
}

#[derive(Resource, Default)]
struct ExtractorIds {
    ids: HashMap<TypeId, usize>,
}

impl ExtractorIds {
    fn register<K: 'static>(&mut self) -> Option<usize> {
        let key = TypeId::of::<K>();
        if self.ids.contains_key(&key) {
            return None;
        }
        let id = self.ids.len();
        self.ids.insert(key, id);
        Some(id)
    }

    fn get<K: 'static>(&self) -> usize {
        self.ids[&TypeId::of::<K>()]
    }
}

struct ComponentExtractorKey<E>(std::marker::PhantomData<fn() -> E>);
struct ManyExtractorKey<E>(std::marker::PhantomData<fn() -> E>);

struct MainQueryState<D: ReadOnlyQueryData, F: QueryFilter> {
    world_id: Option<WorldId>,
    query: Option<QueryState<(Entity, D), F>>,
}

impl<D: ReadOnlyQueryData, F: QueryFilter> Default for MainQueryState<D, F> {
    fn default() -> Self {
        Self {
            world_id: None,
            query: None,
        }
    }
}

impl<D: ReadOnlyQueryData, F: QueryFilter> MainQueryState<D, F> {
    fn get<'a>(&'a mut self, world: &World) -> Option<&'a mut QueryState<(Entity, D), F>> {
        if self.world_id != Some(world.id()) {
            self.world_id = Some(world.id());
            self.query = QueryState::try_new(world);
        } else if self.query.is_none() {
            self.query = QueryState::try_new(world);
        }
        self.query.as_mut()
    }
}

fn collect_ordered_matches<D: ReadOnlyQueryData, F: QueryFilter>(
    query: &mut QueryState<(Entity, D), F>,
    main_world: &World,
    scene_order: &MainSceneOrder,
    matches: &mut Vec<Entity>,
) {
    matches.clear();
    matches.extend(
        query
            .iter(main_world)
            .filter_map(|(entity, _)| scene_order.get(entity).map(|_| entity)),
    );
    matches.sort_unstable_by_key(|entity| scene_order.get(*entity).unwrap());
}

#[derive(SystemParam)]
struct ExtractSystemParams<'w, 's> {
    main_world: Res<'w, MainWorld>,
    scene_order: Res<'w, MainSceneOrder>,
    extractor_ids: Res<'w, ExtractorIds>,
    render_entities: ResMut<'w, RenderEntities>,
    commands: Commands<'w, 's>,
}

/// Remove the extracted bundle of `E` from a render root entity.
fn remove_extracted<E: ExtractComponent>(world: &mut World, entity: Entity) {
    if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
        entity_mut.remove::<E::Out>();
    }
}

fn extract_component_system<E: ExtractComponent>(
    params: ExtractSystemParams,
    mut query_state: Local<MainQueryState<E::QueryData, E::QueryFilter>>,
    mut matches: Local<Vec<Entity>>,
) {
    let ExtractSystemParams {
        main_world,
        scene_order,
        extractor_ids,
        mut render_entities,
        mut commands,
    } = params;
    let main_world = main_world.world();
    let Some(query) = query_state.get(main_world) else {
        return;
    };
    collect_ordered_matches(query, main_world, &scene_order, &mut matches);
    let extractor = extractor_ids.get::<ComponentExtractorKey<E>>();

    for source in matches.iter().copied() {
        let (_, item) = query.get(main_world, source).unwrap();
        let root_order = scene_order.get(source).unwrap();
        let root = render_entities.ensure_root(source, root_order, &mut commands);
        if let Some(bundle) = E::extract_component(item) {
            render_entities.upsert_root(
                RenderItemKey {
                    root,
                    extractor,
                    part: 0,
                },
                root_order,
                bundle,
                remove_extracted::<E>,
                &mut commands,
            );
        }
    }
}

fn extract_many_system<E: ExtractMany>(
    params: ExtractSystemParams,
    mut query_state: Local<MainQueryState<E::QueryData, E::QueryFilter>>,
    mut matches: Local<Vec<Entity>>,
    mut output: Local<ExtractOutput<E::Out>>,
) {
    let ExtractSystemParams {
        main_world,
        scene_order,
        extractor_ids,
        mut render_entities,
        mut commands,
    } = params;
    let main_world = main_world.world();
    let Some(query) = query_state.get(main_world) else {
        return;
    };
    collect_ordered_matches(query, main_world, &scene_order, &mut matches);
    let extractor = extractor_ids.get::<ManyExtractorKey<E>>();

    for source in matches.iter().copied() {
        let (_, item) = query.get(main_world, source).unwrap();
        let root_order = scene_order.get(source).unwrap();
        let root = render_entities.ensure_root(source, root_order, &mut commands);

        output.clear();
        E::extract_many(item, &mut output);
        for (output_order, (part, bundle)) in output.drain().enumerate() {
            render_entities.upsert(
                RenderItemKey {
                    root,
                    extractor,
                    part,
                },
                root_order,
                output_order,
                bundle,
                &mut commands,
            );
        }
    }
}

fn clear_render_entities(world: &mut World) {
    world.resource_scope(
        |world, mut entities: bevy_ecs::world::Mut<RenderEntities>| {
            let render_entities = entities
                .roots
                .values()
                .copied()
                .chain(entities.items.values().map(|slot| slot.entity))
                .collect::<HashSet<_>>();
            for entity in render_entities {
                world.despawn(entity);
            }
            *entities = RenderEntities::default();
        },
    );
}

fn finish_extract(world: &mut World) {
    world.resource_scope(
        |world, mut entities: bevy_ecs::world::Mut<RenderEntities>| {
            let stale_sources = entities
                .roots
                .keys()
                .copied()
                .filter(|source| !entities.seen_roots.contains(source))
                .collect::<Vec<_>>();
            for source in stale_sources {
                let root = entities.roots.remove(&source).unwrap();
                let stale_keys = entities
                    .items
                    .keys()
                    .copied()
                    .filter(|key| key.root == root)
                    .collect::<Vec<_>>();
                for key in stale_keys {
                    if let Some(slot) = entities.items.remove(&key) {
                        // Root-direct outputs die with the root; only
                        // primitive part entities need explicit despawn.
                        if let OnStale::Despawn = slot.on_stale {
                            world.despawn(slot.entity);
                        }
                    }
                }
                world.despawn(root);
            }

            let stale_items = entities
                .items
                .keys()
                .copied()
                .filter(|key| !entities.seen_items.contains(key))
                .collect::<Vec<_>>();
            for key in stale_items {
                if let Some(slot) = entities.items.remove(&key) {
                    match slot.on_stale {
                        OnStale::Despawn => {
                            world.despawn(slot.entity);
                        }
                        OnStale::RemoveComponents(remove_components) => {
                            remove_components(world, slot.entity);
                        }
                    }
                }
            }

            entities
                .frame_items
                .sort_unstable_by_key(|(order, _)| *order);
            let frame_items = mem::take(&mut entities.frame_items);
            entities.ordered_items.clear();
            // A render root appears once per 1:1 extractor that matched it;
            // keep only its first occurrence in the ordered output.
            let mut emitted = HashSet::new();
            for (_, entity) in frame_items {
                if emitted.insert(entity) {
                    world
                        .entity_mut(entity)
                        .insert(RenderItemOrder(entities.ordered_items.len()));
                    entities.ordered_items.push(entity);
                }
            }
        },
    );
}

/// The queued work set for the default view, produced by the Queue stage.
///
/// First version: a single default view containing every extracted render
/// item. Visibility filtering, render phases and multi-view queueing will
/// replace this with per-view work sets.
#[derive(Resource, Default)]
pub struct QueuedFrame {
    /// The camera entity used for the viewport, if any was extracted.
    pub camera: Option<Entity>,
    /// Render-item entities queued for the view, in render order.
    pub items: Vec<Entity>,
}

/// A renderer-owned ECS world containing extracted render entities.
pub struct RenderWorld {
    world: World,
    extract_schedule: Schedule,
    main_world_id: Option<WorldId>,
}

impl Default for RenderWorld {
    fn default() -> Self {
        let mut world = World::new();
        world.init_resource::<RenderEntities>();
        world.init_resource::<ExtractorIds>();
        world.init_resource::<QueuedFrame>();

        let mut render_world = Self {
            world,
            extract_schedule: Schedule::new(ExtractSchedule),
            main_world_id: None,
        };
        render_world.register_component::<CameraFrame>();
        render_world.register_component::<VItem>();
        render_world.register_component::<MeshItem>();
        render_world
    }
}

impl RenderWorld {
    /// Create a render world with the built-in core-item extractors registered.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a query-based 1:1 component extractor.
    pub fn register_component<E: ExtractComponent>(&mut self) {
        let registered = self
            .world
            .resource_mut::<ExtractorIds>()
            .register::<ComponentExtractorKey<E>>()
            .is_some();
        if registered {
            self.extract_schedule
                .add_systems(extract_component_system::<E>);
        }
    }

    /// Register a query-based 1:N component extractor.
    pub fn register_many<E: ExtractMany>(&mut self) {
        let registered = self
            .world
            .resource_mut::<ExtractorIds>()
            .register::<ManyExtractorKey<E>>()
            .is_some();
        if registered {
            self.extract_schedule.add_systems(extract_many_system::<E>);
        }
    }

    /// Access the extraction schedule for custom render integrations.
    pub fn extract_schedule_mut(&mut self) -> &mut Schedule {
        &mut self.extract_schedule
    }

    /// Synchronize and extract the current main-world scene.
    pub fn extract(&mut self, store: &mut CoreItemStore) {
        let scene_order = MainSceneOrder::from_store(store);
        let main_world_id = store.world().id();
        if self.main_world_id != Some(main_world_id) {
            clear_render_entities(&mut self.world);
            self.main_world_id = Some(main_world_id);
        }

        if !store.world().contains_resource::<ScratchMainWorld>() {
            store
                .world_mut()
                .insert_resource(ScratchMainWorld::default());
        }
        let scratch_world = store
            .world_mut()
            .remove_resource::<ScratchMainWorld>()
            .unwrap();
        let main_world = mem::replace(store.world_mut(), scratch_world.0);

        self.world.insert_resource(MainWorld(main_world));
        self.world.insert_resource(scene_order);
        self.world.resource_mut::<RenderEntities>().begin_frame();
        self.extract_schedule.run(&mut self.world);
        finish_extract(&mut self.world);

        self.world.remove_resource::<MainSceneOrder>();
        let main_world = self.world.remove_resource::<MainWorld>().unwrap();
        let scratch_world = mem::replace(store.world_mut(), main_world.0);
        store
            .world_mut()
            .insert_resource(ScratchMainWorld(scratch_world));
    }

    /// Get the underlying render ECS world.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Get the render root corresponding to a main-world entity.
    pub fn root_entity(&self, main_entity: Entity) -> Option<Entity> {
        self.world
            .resource::<RenderEntities>()
            .roots
            .get(&main_entity)
            .copied()
    }

    /// Iterate the extracted render-item entities in render order.
    pub fn ordered_items(&self) -> impl Iterator<Item = Entity> + '_ {
        self.world
            .resource::<RenderEntities>()
            .ordered_items
            .iter()
            .copied()
    }

    /// Queue the default single-view work set for the current frame.
    ///
    /// Queue: every extracted render-item entity joins the default view's work
    /// set. Sort: the work set is ordered by the extraction-time
    /// `RenderItemOrder` (stable scene order); a dedicated Sort stage will
    /// reorder it once render phases, visibility and batch keys exist.
    pub fn queue_default_view(&mut self) {
        let ordered_items = &self.world.resource::<RenderEntities>().ordered_items;
        let camera = ordered_items
            .iter()
            .copied()
            .find(|&entity| self.world.get::<CameraFrame>(entity).is_some());
        let items = ordered_items.clone();
        self.world.insert_resource(QueuedFrame { camera, items });
    }

    /// Borrow the queued work set of the current frame.
    pub fn queued_frame(&self) -> &QueuedFrame {
        self.world.resource::<QueuedFrame>()
    }

    /// The camera frame queued for the default view.
    pub fn queued_camera_frame(&self) -> Option<&CameraFrame> {
        self.queued_frame()
            .camera
            .and_then(|entity| self.world.get::<CameraFrame>(entity))
    }

    /// Iterate vector items in the queued work set, in render order.
    pub fn queued_vitems(&self) -> impl Iterator<Item = &VItem> + Clone {
        self.queued_frame()
            .items
            .iter()
            .filter_map(|&entity| self.world.get::<VItem>(entity))
    }

    /// Iterate mesh items in the queued work set, in render order.
    pub fn queued_mesh_items(&self) -> impl Iterator<Item = &MeshItem> + Clone {
        self.queued_frame()
            .items
            .iter()
            .filter_map(|&entity| self.world.get::<MeshItem>(entity))
    }

    /// Iterate camera frames in extraction order.
    pub fn camera_frames(&self) -> impl Iterator<Item = &CameraFrame> + Clone {
        let entities = self.world.resource::<RenderEntities>();
        entities
            .ordered_items
            .iter()
            .filter_map(|&entity| self.world.get::<CameraFrame>(entity))
    }

    /// Iterate vector items in extraction order.
    pub fn vitems(&self) -> impl Iterator<Item = &VItem> + Clone {
        let entities = self.world.resource::<RenderEntities>();
        entities
            .ordered_items
            .iter()
            .filter_map(|&entity| self.world.get::<VItem>(entity))
    }

    /// Iterate mesh items in extraction order.
    pub fn mesh_items(&self) -> impl Iterator<Item = &MeshItem> + Clone {
        let entities = self.world.resource::<RenderEntities>();
        entities
            .ordered_items
            .iter()
            .filter_map(|&entity| self.world.get::<MeshItem>(entity))
    }

    #[cfg(test)]
    fn item_entities(&self) -> HashMap<RenderItemKey, Entity> {
        self.world
            .resource::<RenderEntities>()
            .items
            .iter()
            .map(|(key, slot)| (*key, slot.entity))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::query::QueryItem;
    use ranim_core::core_item::CoreItem;

    #[derive(Component)]
    struct VItemList(Vec<VItem>);

    struct ExtractVItemList;

    impl ExtractMany for ExtractVItemList {
        type QueryData = &'static VItemList;
        type QueryFilter = ();
        type Out = VItem;

        fn extract_many(
            items: QueryItem<'_, '_, Self::QueryData>,
            output: &mut ExtractOutput<Self::Out>,
        ) {
            for item in &items.0 {
                output.push(item.clone());
            }
        }
    }

    #[derive(Component)]
    struct SourceVItem(VItem);

    #[derive(Component)]
    struct Offset(f32);

    #[derive(Component)]
    struct Visible(bool);

    struct ExtractPositionedVItem;

    impl ExtractComponent for ExtractPositionedVItem {
        type QueryData = (&'static SourceVItem, &'static Offset, &'static Visible);
        type QueryFilter = ();
        type Out = VItem;

        fn extract_component(
            (source, offset, visible): QueryItem<'_, '_, Self::QueryData>,
        ) -> Option<Self::Out> {
            visible.0.then(|| {
                let mut item = source.0.clone();
                item.points[0].x += offset.0;
                item
            })
        }
    }

    #[derive(Component)]
    struct KeyedVItems(Vec<(usize, VItem)>);

    struct ExtractKeyedVItems;

    impl ExtractMany for ExtractKeyedVItems {
        type QueryData = &'static KeyedVItems;
        type QueryFilter = ();
        type Out = VItem;

        fn extract_many(
            items: QueryItem<'_, '_, Self::QueryData>,
            output: &mut ExtractOutput<Self::Out>,
        ) {
            for (part, item) in &items.0 {
                output.emit(*part, item.clone());
            }
        }
    }

    #[test]
    fn render_roots_and_parts_are_reused_and_cleaned_up() {
        let mut left = VItem::default();
        left.points[0].x = -1.0;
        let mut right = VItem::default();
        right.points[0].x = 1.0;

        let mut store = CoreItemStore::new();
        let source = store.insert_item(VItemList(vec![left, right]));
        let mut render_world = RenderWorld::new();
        render_world.register_many::<ExtractVItemList>();
        render_world.extract(&mut store);

        let root = render_world.root_entity(source).unwrap();
        let before = render_world.item_entities();
        let remaining_key = *before
            .keys()
            .find(|key| key.root == root && key.part == 0)
            .unwrap();
        assert_eq!(render_world.vitems().count(), 2);

        store
            .world_mut()
            .get_mut::<VItemList>(source)
            .unwrap()
            .0
            .truncate(1);
        render_world.extract(&mut store);

        assert_eq!(render_world.root_entity(source), Some(root));
        assert_eq!(render_world.vitems().count(), 1);
        assert_eq!(
            render_world.item_entities()[&remaining_key],
            before[&remaining_key]
        );

        store.world_mut().entity_mut(source).remove::<VItemList>();
        render_world.extract(&mut store);

        assert!(render_world.root_entity(source).is_none());
        assert_eq!(render_world.vitems().count(), 0);
    }

    #[test]
    fn built_in_extractors_preserve_main_world_order() {
        let mut first = VItem::default();
        first.points[0].x = 1.0;
        let mut second = VItem::default();
        second.points[0].x = 2.0;

        let mut store = CoreItemStore::new();
        store.update(
            vec![
                ((0, 0), CoreItem::VItem(first.clone())),
                ((1, 0), CoreItem::CameraFrame(CameraFrame::default())),
                ((2, 0), CoreItem::VItem(second.clone())),
            ]
            .into_iter(),
        );

        let mut render_world = RenderWorld::new();
        render_world.extract(&mut store);

        assert_eq!(
            render_world.vitems().cloned().collect::<Vec<_>>(),
            vec![first, second]
        );
        assert_eq!(render_world.camera_frames().count(), 1);
    }

    #[test]
    fn component_extraction_writes_directly_onto_the_render_root() {
        let mut store = CoreItemStore::new();
        let source = store.insert_item(VItem::default());

        let mut render_world = RenderWorld::new();
        render_world.extract(&mut store);

        let root = render_world.root_entity(source).unwrap();
        // The 1:1 output lives on the render root itself, not on a child.
        assert!(render_world.world().get::<VItem>(root).is_some());
        assert!(
            render_world
                .item_entities()
                .values()
                .all(|&entity| entity == root)
        );
        assert_eq!(render_world.vitems().count(), 1);

        // Removing the main-world component removes the extracted output
        // from the root, and the root itself is cleaned up once no extractor
        // matches the main entity anymore.
        store.world_mut().entity_mut(source).remove::<VItem>();
        render_world.extract(&mut store);
        assert!(render_world.root_entity(source).is_none());
        assert_eq!(render_world.vitems().count(), 0);
    }

    #[test]
    fn component_and_many_extractors_can_share_a_render_root() {
        let mut listed = VItem::default();
        listed.points[0].x = 5.0;

        let mut store = CoreItemStore::new();
        let source = store.insert_item((VItem::default(), VItemList(vec![listed])));

        let mut render_world = RenderWorld::new();
        render_world.register_many::<ExtractVItemList>();
        render_world.extract(&mut store);

        let root = render_world.root_entity(source).unwrap();
        assert!(render_world.world().get::<VItem>(root).is_some());
        // root (1:1 core VItem) + one primitive part (1:N list)
        assert_eq!(render_world.vitems().count(), 2);
        // the root appears only once in the ordered output
        assert_eq!(
            render_world
                .ordered_items()
                .filter(|&entity| entity == root)
                .count(),
            1
        );

        // Removing the core VItem only removes the root's own output; the
        // 1:N extractor still matches and keeps the root alive.
        store.world_mut().entity_mut(source).remove::<VItem>();
        render_world.extract(&mut store);

        assert_eq!(render_world.root_entity(source), Some(root));
        assert!(render_world.world().get::<VItem>(root).is_none());
        assert_eq!(render_world.vitems().count(), 1);
        assert_eq!(render_world.vitems().next().unwrap().points[0].x, 5.0);
    }

    #[test]
    fn registered_high_level_item_extracts_without_main_world_render_state() {
        use ranim_items::vitem::geometry::Square;

        let mut store = CoreItemStore::new();
        let source = store.insert_item(Square::new(2.0));
        let mut render_world = RenderWorld::new();
        render_world.register_component::<Square>();
        render_world.extract(&mut store);

        assert!(store.world().get::<Square>(source).is_some());
        let root = render_world.root_entity(source).unwrap();
        assert!(render_world.world().get::<VItem>(root).is_some());
        assert_eq!(render_world.vitems().count(), 1);
    }

    #[test]
    fn query_extractor_reads_multiple_components_and_removes_none_output() {
        let mut item = VItem::default();
        item.points[0].x = 1.0;

        let mut store = CoreItemStore::new();
        let source = store.insert_item((SourceVItem(item), Offset(2.0), Visible(true)));
        let mut render_world = RenderWorld::new();
        render_world.register_component::<ExtractPositionedVItem>();
        render_world.extract(&mut store);

        assert_eq!(render_world.vitems().next().unwrap().points[0].x, 3.0);
        let root = render_world.root_entity(source).unwrap();

        // A `None` output removes the bundle from the root but keeps the root.
        store.world_mut().get_mut::<Visible>(source).unwrap().0 = false;
        render_world.extract(&mut store);
        assert_eq!(render_world.root_entity(source), Some(root));
        assert_eq!(render_world.vitems().count(), 0);

        // Once the query no longer matches, the root itself is cleaned up.
        store.world_mut().entity_mut(source).remove::<Offset>();
        render_world.extract(&mut store);
        assert!(render_world.root_entity(source).is_none());
    }

    #[test]
    fn query_many_uses_explicit_part_keys_across_reordering() {
        let first = VItem::default();
        let mut second = VItem::default();
        second.points[0].x = 2.0;

        let mut store = CoreItemStore::new();
        let source =
            store.insert_item(KeyedVItems(vec![(10, first.clone()), (20, second.clone())]));
        let mut render_world = RenderWorld::new();
        render_world.register_many::<ExtractKeyedVItems>();
        render_world.extract(&mut store);

        let root = render_world.root_entity(source).unwrap();
        let before = render_world.item_entities();
        store
            .world_mut()
            .get_mut::<KeyedVItems>(source)
            .unwrap()
            .0
            .reverse();
        render_world.extract(&mut store);

        let after = render_world.item_entities();
        for part in [10, 20] {
            let key = *before
                .keys()
                .find(|key| key.root == root && key.part == part)
                .unwrap();
            assert_eq!(after[&key], before[&key]);
        }
        assert_eq!(
            render_world
                .vitems()
                .map(|item| item.points[0].x)
                .collect::<Vec<_>>(),
            vec![2.0, 0.0]
        );
    }

    #[test]
    fn query_state_is_rebuilt_when_the_main_world_changes() {
        let mut first_store = CoreItemStore::new();
        first_store.insert_item((SourceVItem(VItem::default()), Offset(1.0), Visible(true)));

        let mut second_item = VItem::default();
        second_item.points[0].x = 10.0;
        let mut second_store = CoreItemStore::new();
        second_store.insert_item((SourceVItem(second_item), Offset(2.0), Visible(true)));

        let mut render_world = RenderWorld::new();
        render_world.register_component::<ExtractPositionedVItem>();
        render_world.extract(&mut first_store);
        assert_eq!(render_world.vitems().next().unwrap().points[0].x, 1.0);

        render_world.extract(&mut second_store);
        assert_eq!(render_world.vitems().next().unwrap().points[0].x, 12.0);
    }

    #[test]
    fn extractor_registration_is_idempotent() {
        let mut store = CoreItemStore::new();
        store.insert_item(VItemList(vec![VItem::default()]));

        let mut render_world = RenderWorld::new();
        render_world.register_many::<ExtractVItemList>();
        render_world.register_many::<ExtractVItemList>();
        render_world.extract(&mut store);

        assert_eq!(render_world.vitems().count(), 1);
    }

    #[test]
    fn extractor_can_be_registered_after_the_schedule_has_run() {
        let mut store = CoreItemStore::new();
        let mut render_world = RenderWorld::new();
        render_world.extract(&mut store);

        store.insert_item(VItemList(vec![VItem::default()]));
        render_world.register_many::<ExtractVItemList>();
        render_world.extract(&mut store);

        assert_eq!(render_world.vitems().count(), 1);
    }
}
