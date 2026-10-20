use bevy_app::{App, Plugin, SubApp};
use bevy_ecs::prelude::*;
use ranim_core::frame::{EvaluatedFrame, FrameItemId};
use ranim_render::extract::{Extract, ExtractContext, ExtractPlugin};
use ranim_render::schedule::RenderSchedules;
use ranim_render::world::SourceOrder;

// Input values need neither Component nor Clone.
struct Sample {
    values: Vec<u32>,
}

#[derive(Resource, Default)]
struct ExtractedSamples(Vec<(FrameItemId, usize, u32)>);

struct SamplePlugin;

impl Plugin for SamplePlugin {
    fn build(&self, app: &mut App) {
        let extract = app.world().resource::<RenderSchedules>().extract;
        app.init_resource::<ExtractedSamples>()
            .add_systems(extract, extract_samples);
    }
}

fn extract_samples(frame: Res<EvaluatedFrame>, mut samples: ResMut<ExtractedSamples>) {
    samples.0 = frame
        .items::<Sample>()
        .iter()
        .flat_map(|item| {
            item.value
                .values
                .iter()
                .map(move |value| (item.id, item.order, *value))
        })
        .collect();
}

#[test]
fn external_plugin_extracts_typed_inputs_into_its_own_resource() {
    let mut app = SubApp::new();
    app.init_resource::<RenderSchedules>();
    let extract = app.world().resource::<RenderSchedules>().extract;
    app.init_resource::<EvaluatedFrame>()
        .add_plugins(SamplePlugin);
    {
        let mut frame = app.world_mut().resource_mut::<EvaluatedFrame>();
        frame.push((1, 0), 42_u64);
        frame.push((2, 0), Sample { values: vec![3, 5] });
        frame.push((3, 0), Sample { values: vec![8] });
    }

    app.world_mut().run_schedule(extract);
    assert_eq!(
        app.world().resource::<ExtractedSamples>().0,
        [((2, 0), 1, 3), ((2, 0), 1, 5), ((3, 0), 2, 8)]
    );
    assert_eq!(app.world().resource::<EvaluatedFrame>().len(), 3);

    app.world_mut().resource_mut::<EvaluatedFrame>().clear();
    app.world_mut().run_schedule(extract);
    assert!(app.world().resource::<ExtractedSamples>().0.is_empty());
}

struct PackedValues(Vec<u32>);

#[derive(Component, Debug, PartialEq)]
struct Value(u32);

#[derive(Component)]
struct FromPacked;

#[derive(Resource, Default)]
struct ExtractCalls(usize);

impl Extract for PackedValues {
    fn extract(&self, ctx: &mut ExtractContext<'_>) {
        ctx.world_mut().resource_mut::<ExtractCalls>().0 += 1;
        for value in &self.0 {
            ctx.entity::<(Value, FromPacked)>()
                .insert((Value(*value), FromPacked));
        }
    }
}

#[test]
fn generic_plugin_extracts_raw_compositions_into_bundles_and_resources() {
    type Input = Vec<Option<PackedValues>>;
    let mut app = SubApp::new();
    app.init_resource::<RenderSchedules>();
    let extract = app.world().resource::<RenderSchedules>().extract;
    app.init_resource::<EvaluatedFrame>()
        .init_resource::<ExtractCalls>()
        .add_plugins(ExtractPlugin::<Input>::default());
    app.world_mut().resource_mut::<EvaluatedFrame>().push(
        (5, 0),
        vec![
            Some(PackedValues(vec![3, 5])),
            None,
            Some(PackedValues(vec![8])),
        ],
    );
    app.world_mut().run_schedule(extract);

    let world = app.world_mut();
    let mut outputs = world
        .query_filtered::<(Entity, &Value, &SourceOrder), With<FromPacked>>()
        .iter(world)
        .map(|(entity, value, order)| (order.part, value.0, entity))
        .collect::<Vec<_>>();
    outputs.sort_by_key(|(part, _, _)| *part);
    assert_eq!(
        outputs
            .iter()
            .map(|(part, value, _)| (*part, *value))
            .collect::<Vec<_>>(),
        [(0, 3), (1, 5), (2, 8)]
    );
    assert_eq!(world.resource::<ExtractCalls>().0, 2);
    assert_eq!(
        world.resource::<EvaluatedFrame>().items::<Input>()[0]
            .value
            .len(),
        3
    );

    world
        .resource_mut::<EvaluatedFrame>()
        .update([((5, 0), Input::new())]);
    world.run_schedule(extract);
    for (_, _, entity) in outputs {
        assert!(!world.entities().contains(entity));
    }
}

#[test]
fn a_leaf_plugin_does_not_implicitly_extract_a_container_batch() {
    let mut app = SubApp::new();
    app.init_resource::<RenderSchedules>();
    let extract = app.world().resource::<RenderSchedules>().extract;
    app.init_resource::<EvaluatedFrame>()
        .init_resource::<ExtractCalls>()
        .add_plugins(ExtractPlugin::<PackedValues>::default());
    app.world_mut()
        .resource_mut::<EvaluatedFrame>()
        .push((1, 0), vec![PackedValues(vec![7])]);
    app.world_mut().run_schedule(extract);
    assert_eq!(app.world().resource::<ExtractCalls>().0, 0);

    app.add_plugins(ExtractPlugin::<Vec<PackedValues>>::default());
    app.world_mut().run_schedule(extract);
    assert_eq!(app.world().resource::<ExtractCalls>().0, 1);
    let world = app.world_mut();
    assert_eq!(world.query::<&Value>().single(world).unwrap(), &Value(7));
}
