//! Composable extraction of evaluated CPU values into the render world.

use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use bevy_app::{App, Plugin};
use bevy_ecs::{prelude::*, world::EntityWorldMut};
use ranim_core::{
    frame::{EvaluatedFrame, FrameItemId},
    glam::DAffine3,
    transformed::Transformed,
};

use crate::{schedule::RenderSchedule, world::SourceOrder};

/// Extract an evaluated value into render-world entities or resources.
///
/// Implementations can recurse into other values using the same context. Each
/// call to [`ExtractContext::entity`] claims the next persistent output of the
/// source frame item. Outputs no longer claimed on the next extraction are
/// despawned automatically. Entities created directly through
/// [`ExtractContext::world_mut`] remain the implementation's responsibility.
pub trait Extract {
    /// Synchronize this value's render-world outputs.
    fn extract(&self, ctx: &mut ExtractContext<'_>);
}

/// Installs extraction for one exact evaluated frame payload type.
///
/// For example, a frame containing `Vec<T>` needs `ExtractPlugin<Vec<T>>`;
/// installing `ExtractPlugin<T>` alone does not visit that batch. Recursive
/// [`Extract`] implementations handle the values inside the registered type.
/// Handwritten systems can also be added to [`RenderSchedule::Extract`].
pub struct ExtractPlugin<T>(PhantomData<fn() -> T>);

impl<T> Default for ExtractPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Extract + Send + Sync + 'static> Plugin for ExtractPlugin<T> {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExtractedEntities<T>>()
            .add_systems(RenderSchedule::Extract, extract_items::<T>);
    }
}

type OutputId = (FrameItemId, usize);

struct OutputEntity {
    entity: Entity,
    bundle_type: TypeId,
}

#[derive(Resource)]
struct ExtractedEntities<T: Send + Sync + 'static> {
    entities: HashMap<OutputId, OutputEntity>,
    seen: HashSet<OutputId>,
    inputs: HashSet<FrameItemId>,
    marker: PhantomData<fn() -> T>,
}

impl<T: Send + Sync + 'static> Default for ExtractedEntities<T> {
    fn default() -> Self {
        Self {
            entities: HashMap::new(),
            seen: HashSet::new(),
            inputs: HashSet::new(),
            marker: PhantomData,
        }
    }
}

/// World access, output identity, order, and inherited transform for extraction.
///
/// A context belongs to one [`ranim_core::frame::FrameItem`]. Recursive values
/// share its output sequence; transform scopes compose without changing that
/// sequence. Input data remains unchanged when a frame is rendered again.
pub struct ExtractContext<'w> {
    world: &'w mut World,
    entities: &'w mut HashMap<OutputId, OutputEntity>,
    seen: &'w mut HashSet<OutputId>,
    source: FrameItemId,
    order: usize,
    next_part: &'w mut usize,
    transform: DAffine3,
}

impl ExtractContext<'_> {
    /// Access the render world to update resources or perform custom operations.
    ///
    /// The current frame and this plugin's identity state are temporarily scoped
    /// out of the world while extracting. The input value is available as `self`
    /// in [`Extract::extract`]. Directly spawned entities are not tracked by the
    /// context; manage their lifetimes in the implementation.
    pub fn world_mut(&mut self) -> &mut World {
        self.world
    }

    /// Claim the source's next persistent output entity.
    ///
    /// `B` identifies the output bundle's shape. The entity is reused while this
    /// shape stays the same; changing it replaces the entity so old components
    /// cannot remain. Values are inserted by the caller. Components managed
    /// outside `B` remain the implementation's responsibility.
    /// An output not claimed on a later frame is despawned.
    pub fn entity<B: Bundle>(&mut self) -> EntityWorldMut<'_> {
        self.next_entity(TypeId::of::<B>())
    }

    /// The accumulated local-to-world transform for the current value.
    pub fn transform(&self) -> DAffine3 {
        self.transform
    }

    /// Extract a subtree under `current * transform`, sharing output identities.
    pub fn with_transform<R>(
        &mut self,
        transform: DAffine3,
        extract: impl FnOnce(&mut ExtractContext<'_>) -> R,
    ) -> R {
        let mut child = ExtractContext {
            world: self.world,
            entities: self.entities,
            seen: self.seen,
            source: self.source,
            order: self.order,
            next_part: self.next_part,
            transform: self.transform * transform,
        };
        extract(&mut child)
    }

    fn next_entity(&mut self, bundle_type: TypeId) -> EntityWorldMut<'_> {
        let part = *self.next_part;
        *self.next_part += 1;
        let id = (self.source, part);
        self.seen.insert(id);

        let output = self.entities.entry(id).or_insert_with(|| OutputEntity {
            entity: self.world.spawn_empty().id(),
            bundle_type,
        });
        if output.bundle_type != bundle_type || !self.world.entities().contains(output.entity) {
            self.world.despawn(output.entity);
            output.entity = self.world.spawn_empty().id();
            output.bundle_type = bundle_type;
        }

        let mut entity = self.world.entity_mut(output.entity);
        let order = SourceOrder {
            order: self.order,
            part,
        };
        if entity.get::<SourceOrder>() != Some(&order) {
            entity.insert(order);
        }
        entity
    }
}

fn extract_items<T: Extract + Send + Sync + 'static>(world: &mut World) {
    let _span = crate::cpu_probe::span("reconcile");
    world.resource_scope(|world, mut state: Mut<ExtractedEntities<T>>| {
        world.resource_scope(|world, frame: Mut<EvaluatedFrame>| {
            let state = &mut *state;
            state.seen.clear();
            state.inputs.clear();
            for item in frame.items::<T>() {
                assert!(
                    state.inputs.insert(item.id),
                    "duplicate frame item id {:?} for `{}`",
                    item.id,
                    std::any::type_name::<T>()
                );
                let mut next_part = 0;
                item.value.extract(&mut ExtractContext {
                    world,
                    entities: &mut state.entities,
                    seen: &mut state.seen,
                    source: item.id,
                    order: item.order,
                    next_part: &mut next_part,
                    transform: DAffine3::IDENTITY,
                });
            }
            state.entities.retain(|id, output| {
                if state.seen.contains(id) {
                    true
                } else {
                    world.despawn(output.entity);
                    false
                }
            });
        });
    });
}

impl<T: Extract, G: Clone + Into<DAffine3>> Extract for Transformed<T, G> {
    fn extract(&self, ctx: &mut ExtractContext<'_>) {
        ctx.with_transform(self.transform.clone().into(), |ctx| self.inner.extract(ctx));
    }
}

impl<T: Extract> Extract for Vec<T> {
    fn extract(&self, ctx: &mut ExtractContext<'_>) {
        for item in self {
            item.extract(ctx);
        }
    }
}

impl<T: Extract> Extract for Option<T> {
    fn extract(&self, ctx: &mut ExtractContext<'_>) {
        if let Some(item) = self {
            item.extract(ctx);
        }
    }
}

impl<T: Extract, const N: usize> Extract for [T; N] {
    fn extract(&self, ctx: &mut ExtractContext<'_>) {
        for item in self {
            item.extract(ctx);
        }
    }
}

impl Extract for () {
    fn extract(&self, _ctx: &mut ExtractContext<'_>) {}
}

macro_rules! impl_extract_for_tuple {
    ($(($T:ident, $value:ident)),*) => {
        impl<$($T: Extract),*> Extract for ($($T,)*) {
            fn extract(&self, ctx: &mut ExtractContext<'_>) {
                let ($($value,)*) = self;
                $($value.extract(ctx);)*
            }
        }
    };
}

variadics_please::all_tuples!(impl_extract_for_tuple, 1, 15, T, value);

#[cfg(test)]
mod tests {
    use bevy_app::SubApp;
    use bevy_ecs::{query::Changed, schedule::ScheduleLabel};
    use ranim_core::glam::{DMat4, DVec3};

    use super::*;

    #[derive(Component, Default, Clone, Debug, PartialEq)]
    struct TestItem {
        value: f32,
        transform: ranim_core::glam::Mat4,
    }

    impl Extract for TestItem {
        fn extract(&self, ctx: &mut ExtractContext<'_>) {
            let mut output = self.clone();
            output.transform = DMat4::from(ctx.transform()).as_mat4() * output.transform;
            let mut entity = ctx.entity::<Self>();
            if entity.get::<Self>() != Some(&output) {
                entity.insert(output);
            }
        }
    }

    #[derive(Component, Default, Clone)]
    struct OtherItem {
        _value: u8,
    }

    impl Extract for OtherItem {
        fn extract(&self, ctx: &mut ExtractContext<'_>) {
            ctx.entity::<Self>().insert(self.clone());
        }
    }

    fn extraction_app<T: Extract + Send + Sync + 'static>() -> SubApp {
        let mut app = SubApp::new();
        app.update_schedule = Some(RenderSchedule::Extract.intern());
        app.init_resource::<EvaluatedFrame>()
            .add_plugins(ExtractPlugin::<T>::default());
        app
    }

    fn entity<T: Send + Sync + 'static>(app: &SubApp, source: FrameItemId, part: usize) -> Entity {
        app.world().resource::<ExtractedEntities<T>>().entities[&(source, part)].entity
    }

    #[test]
    fn raw_compositions_extract_in_order_and_empty_vectors_remove_outputs() {
        type Input = Vec<(TestItem, Option<OtherItem>)>;
        let mut app = extraction_app::<Input>();
        app.world_mut().resource_mut::<EvaluatedFrame>().push(
            (7, 0),
            vec![
                (TestItem::default(), Some(OtherItem::default())),
                (TestItem::default(), None),
            ],
        );
        app.update();
        for part in 0..3 {
            assert_eq!(
                app.world()
                    .get::<SourceOrder>(entity::<Input>(&app, (7, 0), part)),
                Some(&SourceOrder { order: 0, part }),
            );
        }
        let previous = entity::<Input>(&app, (7, 0), 0);
        app.world_mut()
            .resource_mut::<EvaluatedFrame>()
            .update([((7, 0), Input::new())]);
        app.update();
        assert!(!app.world().entities().contains(previous));
        assert!(
            app.world()
                .resource::<ExtractedEntities<Input>>()
                .entities
                .is_empty()
        );
    }

    #[test]
    fn unchanged_component_outputs_keep_entities_and_changed_tracking() {
        #[derive(Resource, Default)]
        struct Changes(Vec<usize>);

        let mut app = extraction_app::<Vec<TestItem>>();
        app.init_resource::<Changes>().add_systems(
            RenderSchedule::Extract,
            (|items: Query<Entity, Changed<TestItem>>, mut changes: ResMut<Changes>| {
                changes.0.push(items.iter().count());
            })
            .after(extract_items::<Vec<TestItem>>),
        );
        app.world_mut()
            .resource_mut::<EvaluatedFrame>()
            .push((1, 0), vec![TestItem::default()]);
        app.update();
        let original = entity::<Vec<TestItem>>(&app, (1, 0), 0);
        app.update();
        assert_eq!(entity::<Vec<TestItem>>(&app, (1, 0), 0), original);
        assert_eq!(app.world().resource::<Changes>().0, [1, 0]);

        let changed = TestItem {
            value: 2.0,
            ..Default::default()
        };
        app.world_mut()
            .resource_mut::<EvaluatedFrame>()
            .update([((1, 0), vec![changed])]);
        app.update();
        assert_eq!(app.world().resource::<Changes>().0, [1, 0, 1]);
    }

    #[test]
    fn absent_optional_output_cannot_leave_a_previous_bundle_shape() {
        type Input = (Option<TestItem>, OtherItem);
        let mut app = extraction_app::<Input>();
        app.world_mut()
            .resource_mut::<EvaluatedFrame>()
            .push((1, 0), (Some(TestItem::default()), OtherItem::default()));
        app.update();
        let first = entity::<Input>(&app, (1, 0), 0);
        let second = entity::<Input>(&app, (1, 0), 1);

        app.world_mut()
            .resource_mut::<EvaluatedFrame>()
            .update([((1, 0), (None::<TestItem>, OtherItem::default()))]);
        app.update();
        let current = entity::<Input>(&app, (1, 0), 0);
        assert!(!app.world().entities().contains(first));
        assert!(!app.world().entities().contains(second));
        assert!(app.world().get::<OtherItem>(current).is_some());
        assert!(app.world().get::<TestItem>(current).is_none());
    }

    #[test]
    fn nested_transforms_compose_during_extraction_without_changing_raw_input() {
        type Input = Transformed<Transformed<TestItem, DAffine3>, DAffine3>;
        let outer = DAffine3::from_translation(DVec3::X * 3.0);
        let inner = DAffine3::from_scale(DVec3::splat(2.0));
        let input = Transformed::new(Transformed::new(TestItem::default(), inner), outer);
        let mut app = extraction_app::<Input>();
        app.world_mut()
            .resource_mut::<EvaluatedFrame>()
            .push((1, 0), input.clone());
        app.update();
        app.update();

        let output = app
            .world()
            .get::<TestItem>(entity::<Input>(&app, (1, 0), 0))
            .unwrap();
        assert_eq!(output.transform, DMat4::from(outer * inner).as_mat4());
        assert_eq!(
            app.world().resource::<EvaluatedFrame>().items::<Input>()[0].value,
            input
        );
    }

    #[test]
    fn a_missing_whole_batch_cleans_only_its_own_entities() {
        let mut app = extraction_app::<TestItem>();
        app.add_plugins(ExtractPlugin::<OtherItem>::default());
        app.world_mut()
            .resource_mut::<EvaluatedFrame>()
            .push((1, 0), TestItem::default());
        app.world_mut()
            .resource_mut::<EvaluatedFrame>()
            .push((2, 0), OtherItem::default());
        app.update();
        let removed = entity::<TestItem>(&app, (1, 0), 0);
        let preserved = entity::<OtherItem>(&app, (2, 0), 0);

        let mut frame = EvaluatedFrame::new();
        frame.push((2, 0), OtherItem::default());
        app.insert_resource(frame);
        app.update();
        assert!(!app.world().entities().contains(removed));
        assert_eq!(entity::<OtherItem>(&app, (2, 0), 0), preserved);
    }

    #[test]
    #[should_panic(expected = "duplicate frame item id (1, 0)")]
    fn duplicate_input_ids_are_rejected() {
        let mut app = extraction_app::<TestItem>();
        app.world_mut()
            .resource_mut::<EvaluatedFrame>()
            .update([((1, 0), TestItem::default()), ((1, 0), TestItem::default())]);
        app.update();
    }
}
