mod common;

use bevy::prelude::*;

fn main() {
    common::run_example(
        common::ExampleConfig::new("Ranim Bevy: Animated Fox Halo", "fox_halo", 4.0)
            .with_asset_path(common::fox_halo::ASSET_PATH)
            .with_output_name("fox_halo_bevy"),
        build_scene,
    );
}

fn build_scene(app: &mut App) {
    app.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 1200.0,
        ..default()
    })
    .add_systems(
        Startup,
        (
            common::fox_halo::setup_world,
            common::fox_halo::setup_fox,
            common::fox_halo::setup_halo,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (
            common::fox_halo::animate_halo_segments,
            common::fox_halo::orbit_camera,
        ),
    );
}
