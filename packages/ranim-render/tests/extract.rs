use bevy_app::{App, Plugin, SubApp};
use bevy_ecs::prelude::*;
use ranim_core::frame::{EvaluatedFrame, FrameItemId};
use ranim_render::schedule::RenderExtract;

// Input values need neither Component nor Clone.
struct Sample {
    values: Vec<u32>,
}

#[derive(Resource, Default)]
struct ExtractedSamples(Vec<(FrameItemId, usize, u32)>);

struct SamplePlugin;

impl Plugin for SamplePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExtractedSamples>()
            .add_systems(RenderExtract, extract_samples);
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
    app.init_resource::<EvaluatedFrame>()
        .add_plugins(SamplePlugin);
    {
        let mut frame = app.world_mut().resource_mut::<EvaluatedFrame>();
        frame.push((1, 0), 42_u64);
        frame.push((2, 0), Sample { values: vec![3, 5] });
        frame.push((3, 0), Sample { values: vec![8] });
    }

    app.world_mut().run_schedule(RenderExtract);
    assert_eq!(
        app.world().resource::<ExtractedSamples>().0,
        [((2, 0), 1, 3), ((2, 0), 1, 5), ((3, 0), 2, 8)]
    );
    assert_eq!(app.world().resource::<EvaluatedFrame>().len(), 3);

    app.world_mut().resource_mut::<EvaluatedFrame>().clear();
    app.world_mut().run_schedule(RenderExtract);
    assert!(app.world().resource::<ExtractedSamples>().0.is_empty());
}
