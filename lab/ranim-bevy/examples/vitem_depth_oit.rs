mod common;

use bevy::{
    core_pipeline::oit::OrderIndependentTransparencySettings,
    prelude::*,
};
use ranim_bevy::RanimVItem;

fn main() {
    common::run_example(
        common::ExampleConfig::new("Ranim Bevy: Depth And OIT", "vitem_depth_oit", 6.0),
        build_scene,
    );
}

fn build_scene(app: &mut App) {
    app.add_systems(Startup, setup).add_systems(Update, orbit);
}

#[derive(Component)]
struct OrbitRoot;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.4, 8.2).looking_at(Vec3::ZERO, Vec3::Y),
        OrderIndependentTransparencySettings::default(),
        Msaa::Off,
    ));
    common::light(&mut commands);

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.6, 2.6, 0.22))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.20, 0.22, 0.27),
            perceptual_roughness: 0.78,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, -0.62),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(3.9, 3.9).mesh())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.95, 0.70, 0.14, 0.28),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.08, -0.08, 0.26).with_rotation(Quat::from_rotation_y(0.42)),
    ));

    commands
        .spawn((OrbitRoot, Transform::default(), Visibility::default()))
        .with_children(|parent| {
            parent.spawn((
                RanimVItem::new(common::star(
                    7,
                    1.85,
                    1.14,
                    0.0,
                    [0.05, 0.62, 0.88, 0.76],
                    [1.0, 1.0, 1.0, 0.96],
                    0.05,
                )),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
            parent.spawn((
                RanimVItem::new(common::regular_polygon(
                    6,
                    1.25,
                    [0.92, 0.18, 0.36, 0.62],
                    [0.95, 0.86, 0.30, 0.96],
                    0.04,
                )),
                Transform::from_xyz(0.0, 0.0, 0.55)
                    .with_rotation(Quat::from_rotation_y(-0.58)),
            ));
        });
}

fn orbit(time: Res<Time>, mut query: Query<&mut Transform, With<OrbitRoot>>) {
    let t = time.elapsed_secs();
    for mut transform in &mut query {
        transform.rotation = Quat::from_rotation_y(t * 0.38) * Quat::from_rotation_x(0.28);
    }
}
