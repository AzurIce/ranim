use std::{
    any::TypeId,
    cell::RefCell,
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use bevy_ecs::{component::Component, entity::Entity, world::World};

use crate::{
    animation::{AnimationCell, CoreItemAnimation},
    core_item::{AnyExtractCoreItem, CoreItem, mesh_item::MeshItem, vitem::VItem},
    prelude::CameraFrame,
};

/// A store of animations
///
/// It has interior mutability, because when pushing an animation into it, we
/// need to return a reference to the animation, which is bound to the store's lifetime.
///
/// To allow the mutation, we use a `RefCell<Vec<Box<dyn AnyAnimation>>>` in its inner.
///
/// # Safety Contract
///
/// The following invariants must be maintained:
///
/// - **No mutation after push**: Once an animation is pushed into the store, it should never
///   be mutated or removed. The only allowed mutation is pushing new animations into the store.
///
/// - **No Vec reallocation issues**: The returned references from `push_eval_dynamic` point directly
///   to the heap-allocated `AnimationCell<T>` data inside the `Box<dyn AnyAnimation>`. Even if the
///   `Vec` reallocates (which moves the `Box`es), the heap data itself doesn't move, so the pointers
///   remain valid. This is safe because `Box` owns heap-allocated data, and the data doesn't move
///   when the `Box` is moved within the `Vec`.
#[derive(Default)]
pub struct AnimationStore {
    anims: RefCell<Vec<Box<dyn CoreItemAnimation>>>,
}

impl AnimationStore {
    /// Create a new store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push an `AnimationCell<T>` into the store and return a reference to it.
    ///
    /// The returned reference is bound to `&self`'s lifetime, which means it will be invalidated
    /// when the store is dropped. Since we use `RefCell` for interior mutability, we can modify
    /// the internal `Vec` while holding a shared reference `&self`.
    ///
    /// # Safety
    ///
    /// This function uses unsafe code to return a reference that outlives the `RefCell` borrow.
    /// The safety relies on the following guarantees:
    ///
    /// 1. **Pointer validity**: The raw pointer `ptr` points to the heap-allocated `AnimationCell<T>`
    ///    that is now owned by the `Vec<Box<dyn AnyAnimation>>` inside `self.anims`.
    ///
    /// 2. **Memory layout**: When we coerce `Box<AnimationCell<T>>` to `Box<dyn AnyAnimation>`,
    ///    only the vtable pointer changes. The data pointer (pointing to the actual `AnimationCell<T>`
    ///    on the heap) remains the same, so `ptr` is still valid.
    ///
    /// 3. **Vec reallocation safety**: Even if the `Vec` reallocates (which moves the `Box`es),
    ///    the heap-allocated `AnimationCell<T>` data inside each `Box` does not move. The pointer
    ///    `ptr` points directly to this heap data, not to the `Box` itself, so it remains valid
    ///    regardless of `Vec` reallocations. This is a key property of `Box`: moving the `Box`
    ///    doesn't move the data it points to on the heap.
    ///
    /// 4. **Lifetime binding**: The returned reference `&AnimationCell<T>` has a lifetime that is
    ///    inferred from `&self`, ensuring it cannot outlive the store. This is enforced by Rust's
    ///    borrow checker.
    ///
    /// 5. **No mutation after push**: Once pushed, the animation is never mutated or removed,
    ///    so the pointer remains valid for the lifetime of the store.
    pub fn push_animation<T: AnyExtractCoreItem>(
        &self,
        anim: AnimationCell<T>,
    ) -> &AnimationCell<T> {
        let boxed = Box::new(anim);

        // Get a raw pointer to the heap-allocated AnimationCell<T> before converting
        let ptr = Box::into_raw(boxed);
        // Reconstruct as Box<AnimationCell<T>>, then coerce to Box<dyn AnyAnimation>
        // This ensures the vtable is properly set up
        let boxed_concrete: Box<AnimationCell<T>> = unsafe { Box::from_raw(ptr) };
        let boxed_trait: Box<dyn CoreItemAnimation> = boxed_concrete;
        self.anims.borrow_mut().push(boxed_trait);
        // SAFETY: See function documentation for detailed safety guarantees.
        // In summary: ptr points to memory owned by the Vec, the Vec won't reallocate
        // until capacity is exceeded (and we're pushing one element), and the returned
        // reference's lifetime is bound to &self, ensuring it cannot outlive the store.
        unsafe { &*ptr }
    }
}

/// The source id attached to an evaluated core item.
pub type CoreItemSourceId = (usize, usize);

/// The stable identity of one extracted core item occurrence.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoreItemKey {
    /// The timeline and animation that emitted the item.
    pub source: CoreItemSourceId,
    /// The occurrence index within all items emitted by the source.
    pub part: usize,
}

/// The order of a core item in the current evaluated frame.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreItemOrder(pub usize);

/// Extract a main-world item into one or more render-world items.
pub trait ExtractToRenderWorld: Component {
    /// The component stored on extracted render entities.
    type RenderItem: Component;

    /// Extract render items from this main-world item.
    fn extract_to_render_world(&self, output: &mut Vec<Self::RenderItem>);
}

impl ExtractToRenderWorld for CameraFrame {
    type RenderItem = CameraFrame;

    fn extract_to_render_world(&self, output: &mut Vec<Self::RenderItem>) {
        output.push(self.clone());
    }
}

impl ExtractToRenderWorld for VItem {
    type RenderItem = VItem;

    fn extract_to_render_world(&self, output: &mut Vec<Self::RenderItem>) {
        output.push(self.clone());
    }
}

impl ExtractToRenderWorld for MeshItem {
    type RenderItem = MeshItem;

    fn extract_to_render_world(&self, output: &mut Vec<Self::RenderItem>) {
        output.push(self.clone());
    }
}

/// The identity of an extracted render item occurrence.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderItemKey {
    /// The main-world entity that emitted the item.
    pub source: Entity,
    /// The registered extractor that emitted the item.
    pub extractor: usize,
    /// The occurrence index emitted by the extractor.
    pub part: usize,
}

/// The main-world entity that produced a render entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExtractedFrom(pub Entity);

/// The order of an item in the extracted render world.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderItemOrder(pub usize);

/// The ECS world consumed by render preparation.
#[derive(Default)]
pub struct RenderWorld {
    world: World,
    entities: HashMap<RenderItemKey, Entity>,
    ordered_entities: Vec<Entity>,
}

impl RenderWorld {
    /// Get the underlying ECS world.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Iterate camera frames in extraction order.
    pub fn camera_frames(&self) -> impl Iterator<Item = &CameraFrame> + Clone {
        self.ordered_entities
            .iter()
            .filter_map(|&entity| self.world.get::<CameraFrame>(entity))
    }

    /// Iterate vector items in extraction order.
    pub fn vitems(&self) -> impl Iterator<Item = &VItem> + Clone {
        self.ordered_entities
            .iter()
            .filter_map(|&entity| self.world.get::<VItem>(entity))
    }

    /// Iterate mesh items in extraction order.
    pub fn mesh_items(&self) -> impl Iterator<Item = &MeshItem> + Clone {
        self.ordered_entities
            .iter()
            .filter_map(|&entity| self.world.get::<MeshItem>(entity))
    }

    fn upsert<T: Component>(&mut self, key: RenderItemKey, order: usize, item: T) -> Entity {
        if let Some(&entity) = self.entities.get(&key) {
            self.world
                .entity_mut(entity)
                .insert((RenderItemOrder(order), item));
            entity
        } else {
            let entity = self
                .world
                .spawn((key, ExtractedFrom(key.source), RenderItemOrder(order), item))
                .id();
            self.entities.insert(key, entity);
            entity
        }
    }

    fn finish_extract(&mut self, seen: &HashSet<RenderItemKey>, ordered_entities: Vec<Entity>) {
        let stale = self
            .entities
            .keys()
            .copied()
            .filter(|key| !seen.contains(key))
            .collect::<Vec<_>>();
        for key in stale {
            if let Some(entity) = self.entities.remove(&key) {
                self.world.despawn(entity);
            }
        }
        self.ordered_entities = ordered_entities;
    }
}

trait ErasedExtractor: Send + Sync {
    fn source_type_id(&self) -> TypeId;
    fn extract_entity(
        &self,
        extractor: usize,
        main_world: &World,
        source: Entity,
        render_world: &mut RenderWorld,
        seen: &mut HashSet<RenderItemKey>,
        ordered_entities: &mut Vec<Entity>,
    );
}

struct TypedExtractor<T>(PhantomData<fn() -> T>);

impl<T> Default for TypedExtractor<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: ExtractToRenderWorld> ErasedExtractor for TypedExtractor<T> {
    fn source_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn extract_entity(
        &self,
        extractor: usize,
        main_world: &World,
        source: Entity,
        render_world: &mut RenderWorld,
        seen: &mut HashSet<RenderItemKey>,
        ordered_entities: &mut Vec<Entity>,
    ) {
        let Some(item) = main_world.get::<T>(source) else {
            return;
        };

        let mut output = Vec::new();
        item.extract_to_render_world(&mut output);
        for (part, item) in output.into_iter().enumerate() {
            let key = RenderItemKey {
                source,
                extractor,
                part,
            };
            let entity = render_world.upsert(key, ordered_entities.len(), item);
            seen.insert(key);
            ordered_entities.push(entity);
        }
    }
}

/// An ECS-backed store with separate main and render worlds.
pub struct CoreItemStore {
    world: World,
    entities: HashMap<CoreItemKey, Entity>,
    evaluated_entities: Vec<Entity>,
    item_entities: Vec<Entity>,
    render_world: RenderWorld,
    extractors: Vec<Box<dyn ErasedExtractor>>,
    extractor_types: HashSet<TypeId>,
}

impl Default for CoreItemStore {
    fn default() -> Self {
        let mut store = Self {
            world: World::new(),
            entities: HashMap::new(),
            evaluated_entities: Vec::new(),
            item_entities: Vec::new(),
            render_world: RenderWorld::default(),
            extractors: Vec::new(),
            extractor_types: HashSet::new(),
        };
        store.register_item::<CameraFrame>();
        store.register_item::<VItem>();
        store.register_item::<MeshItem>();
        store
    }
}

impl CoreItemStore {
    /// Create an empty store
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the underlying ECS world.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Get the underlying ECS world mutably.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Get the extracted render world.
    pub fn render_world(&self) -> &RenderWorld {
        &self.render_world
    }

    /// Register an item component and its extractor.
    pub fn register_item<T: ExtractToRenderWorld>(&mut self) {
        let type_id = TypeId::of::<T>();
        if self.extractor_types.insert(type_id) {
            let extractor = TypedExtractor::<T>::default();
            debug_assert_eq!(extractor.source_type_id(), type_id);
            self.extractors.push(Box::new(extractor));
        }
    }

    /// Insert an arbitrary renderable item component into the main world.
    pub fn insert_item<T: ExtractToRenderWorld>(&mut self, item: T) -> Entity {
        self.register_item::<T>();
        let entity = self.world.spawn(item).id();
        self.item_entities.push(entity);
        self.extract();
        entity
    }

    /// Extract all registered main-world item components into the render world.
    pub fn extract(&mut self) {
        self.item_entities
            .retain(|&entity| self.world.get_entity(entity).is_ok());
        self.evaluated_entities
            .retain(|&entity| self.world.get_entity(entity).is_ok());

        let mut seen = HashSet::new();
        let mut ordered_entities = Vec::new();
        let sources = self
            .item_entities
            .iter()
            .chain(&self.evaluated_entities)
            .copied()
            .collect::<Vec<_>>();

        for source in sources {
            for (extractor, registered) in self.extractors.iter().enumerate() {
                registered.extract_entity(
                    extractor,
                    &self.world,
                    source,
                    &mut self.render_world,
                    &mut seen,
                    &mut ordered_entities,
                );
            }
        }
        self.render_world.finish_extract(&seen, ordered_entities);
    }

    /// Get the number of core item entities.
    pub fn len(&self) -> usize {
        self.item_entities.len() + self.evaluated_entities.len()
    }

    /// Whether the store contains no core item entities.
    pub fn is_empty(&self) -> bool {
        self.item_entities.is_empty() && self.evaluated_entities.is_empty()
    }

    /// Iterate camera frames in evaluated frame order.
    pub fn camera_frames(&self) -> impl Iterator<Item = &CameraFrame> + Clone {
        self.render_world.camera_frames()
    }

    /// Iterate vector items in evaluated frame order.
    pub fn vitems(&self) -> impl Iterator<Item = &VItem> + Clone {
        self.render_world.vitems()
    }

    /// Iterate mesh items in evaluated frame order.
    pub fn mesh_items(&self) -> impl Iterator<Item = &MeshItem> + Clone {
        self.render_world.mesh_items()
    }

    /// Update the inner world with a fully evaluated frame.
    pub fn update(&mut self, items: impl Iterator<Item = (CoreItemSourceId, CoreItem)>) {
        let mut occurrences = HashMap::<CoreItemSourceId, usize>::new();
        let mut seen = HashSet::new();
        let mut evaluated_entities = Vec::new();

        for (order, (source, item)) in items.enumerate() {
            let part = occurrences.entry(source).or_default();
            let key = CoreItemKey {
                source,
                part: *part,
            };
            *part += 1;

            let entity = if let Some(&entity) = self.entities.get(&key) {
                let mut entity_mut = self.world.entity_mut(entity);
                entity_mut.insert(CoreItemOrder(order));
                match item {
                    CoreItem::CameraFrame(item) => {
                        entity_mut.remove::<VItem>();
                        entity_mut.remove::<MeshItem>();
                        entity_mut.insert(item)
                    }
                    CoreItem::VItem(item) => {
                        entity_mut.remove::<CameraFrame>();
                        entity_mut.remove::<MeshItem>();
                        entity_mut.insert(item)
                    }
                    CoreItem::MeshItem(item) => {
                        entity_mut.remove::<CameraFrame>();
                        entity_mut.remove::<VItem>();
                        entity_mut.insert(item)
                    }
                };
                entity
            } else {
                let entity = match item {
                    CoreItem::CameraFrame(item) => {
                        self.world.spawn((key, CoreItemOrder(order), item)).id()
                    }
                    CoreItem::VItem(item) => {
                        self.world.spawn((key, CoreItemOrder(order), item)).id()
                    }
                    CoreItem::MeshItem(item) => {
                        self.world.spawn((key, CoreItemOrder(order), item)).id()
                    }
                };
                self.entities.insert(key, entity);
                entity
            };

            seen.insert(key);
            evaluated_entities.push(entity);
        }

        let stale = self
            .entities
            .keys()
            .copied()
            .filter(|key| !seen.contains(key))
            .collect::<Vec<_>>();
        for key in stale {
            if let Some(entity) = self.entities.remove(&key) {
                self.world.despawn(entity);
            }
        }

        self.evaluated_entities = evaluated_entities;
        self.extract();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_item_store_reconciles_entities_and_preserves_order() {
        let source = (3, 7);
        let mut first_vitem = VItem::default();
        first_vitem.points[0].x = 1.0;
        let mut second_vitem = VItem::default();
        second_vitem.points[0].x = 2.0;

        let mut store = CoreItemStore::new();
        store.update(
            vec![
                (source, CoreItem::VItem(first_vitem.clone())),
                (source, CoreItem::VItem(second_vitem.clone())),
                ((4, 0), CoreItem::CameraFrame(CameraFrame::default())),
            ]
            .into_iter(),
        );

        let first_key = CoreItemKey { source, part: 0 };
        let second_key = CoreItemKey { source, part: 1 };
        let first_entity = store.entities[&first_key];
        let second_entity = store.entities[&second_key];

        assert_eq!(store.len(), 3);
        assert_eq!(
            store.vitems().cloned().collect::<Vec<_>>(),
            vec![first_vitem.clone(), second_vitem]
        );
        assert_eq!(store.camera_frames().count(), 1);

        first_vitem.points[0].x = 9.0;
        store.update(
            vec![
                ((4, 0), CoreItem::CameraFrame(CameraFrame::default())),
                (source, CoreItem::VItem(first_vitem.clone())),
            ]
            .into_iter(),
        );

        assert_eq!(store.entities[&first_key], first_entity);
        assert!(!store.entities.contains_key(&second_key));
        assert_eq!(store.vitems().next(), Some(&first_vitem));
        assert_eq!(store.camera_frames().count(), 1);
        assert_eq!(store.world.get::<CoreItemOrder>(first_entity).unwrap().0, 1);
        assert_ne!(first_entity, second_entity);
    }

    #[test]
    fn core_item_store_reuses_entity_when_component_kind_changes() {
        let source = (1, 2);
        let key = CoreItemKey { source, part: 0 };
        let mut store = CoreItemStore::new();
        store.update(std::iter::once((source, CoreItem::VItem(VItem::default()))));
        let entity = store.entities[&key];

        store.update(std::iter::once((
            source,
            CoreItem::MeshItem(MeshItem::default()),
        )));

        assert_eq!(store.entities[&key], entity);
        assert!(store.world.get::<VItem>(entity).is_none());
        assert!(store.world.get::<MeshItem>(entity).is_some());
    }

    #[test]
    fn arbitrary_component_extracts_to_independent_render_entities() {
        #[derive(Component)]
        struct VItemPair([VItem; 2]);

        impl ExtractToRenderWorld for VItemPair {
            type RenderItem = VItem;

            fn extract_to_render_world(&self, output: &mut Vec<Self::RenderItem>) {
                output.extend(self.0.iter().cloned());
            }
        }

        let mut left = VItem::default();
        left.points[0].x = -1.0;
        let mut right = VItem::default();
        right.points[0].x = 1.0;

        let mut store = CoreItemStore::new();
        let source = store.insert_item(VItemPair([left.clone(), right.clone()]));

        assert!(store.world().get::<VItemPair>(source).is_some());
        assert_eq!(
            store.vitems().cloned().collect::<Vec<_>>(),
            vec![left, right]
        );

        let before = store
            .render_world
            .entities
            .iter()
            .filter(|(key, _)| key.source == source)
            .map(|(key, entity)| (*key, *entity))
            .collect::<HashMap<_, _>>();

        store.world_mut().get_mut::<VItemPair>(source).unwrap().0[0].points[0].x = -2.0;
        store.extract();

        let after = store
            .render_world
            .entities
            .iter()
            .filter(|(key, _)| key.source == source)
            .map(|(key, entity)| (*key, *entity))
            .collect::<HashMap<_, _>>();
        assert_eq!(before, after);
        assert_eq!(store.vitems().next().unwrap().points[0].x, -2.0);

        store.world_mut().despawn(source);
        store.extract();
        assert_eq!(store.vitems().count(), 0);
        assert!(
            store
                .render_world
                .entities
                .keys()
                .all(|key| key.source != source)
        );
    }

    #[test]
    fn test_animation_store() {
        use crate::animation::Eval;
        use std::marker::PhantomData;
        #[derive(Default)]
        struct A<T: Default> {
            _phantom: PhantomData<T>,
        }
        impl<T: Default> Eval<T> for A<T> {
            fn eval_alpha(&self, _alpha: f64) -> T {
                T::default()
            }
        }

        let store = AnimationStore::new();
        let anim = store.push_animation(A::<VItem>::default().into_animation_cell());
        // drop(store); // This should cause a compile error because anim's lifetime is tied to store
        assert_eq!(anim.eval_alpha(0.0), VItem::default());
        assert_eq!(
            anim.eval_alpha_core_item(0.0),
            vec![CoreItem::VItem(VItem::default())]
        );
        drop(store);
    }
}
