use std::f64::consts::TAU;

use bevy::{
    core_pipeline::{oit::OrderIndependentTransparencySettings, prepass::DepthPrepass},
    prelude::*,
};
use ranim_bevy::{RanimBevyPlugin, RanimVItem};
use ranim_core::{
    VItem,
    components::{rgba::Rgba, width::Width},
    glam::{dvec3, vec4},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ranim Bevy Demo".to_string(),
                resolution: (1280, 720).into(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(RanimBevyPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, animate_vitem)
        .run();
}

#[derive(Component)]
struct AnimatedRanimShape;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        OrderIndependentTransparencySettings::default(),
        DepthPrepass,
        Msaa::Off,
    ));
    commands.spawn((
        PointLight {
            intensity: 1_500.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(3.0, 4.0, 6.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.4, 2.4, 0.18))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.20, 0.24),
            perceptual_roughness: 0.72,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, -0.55),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(3.8, 3.8).mesh())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.75, 0.18, 0.28),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.15, -0.1, 0.25)
            .with_rotation(Quat::from_rotation_y(0.42)),
    ));
    commands.spawn((
        AnimatedRanimShape,
        RanimVItem::new(make_shape(0.0)),
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(0.76)),
    ));
}

fn animate_vitem(time: Res<Time>, mut query: Query<&mut RanimVItem, With<AnimatedRanimShape>>) {
    let t = time.elapsed_secs_f64();
    for mut shape in &mut query {
        shape.item = make_shape(t);
    }
}

fn make_shape(t: f64) -> VItem {
    let count = 7;
    let pulse = 0.22 * (t * 1.4).sin();
    let twist = t * 0.55;
    let mut points = Vec::with_capacity(count * 2 + 1);

    for idx in 0..count {
        let a = idx as f64 / count as f64 * TAU + twist;
        let next = (idx + 1) as f64 / count as f64 * TAU + twist;
        let r = 2.0 + pulse + 0.22 * (t * 2.1 + idx as f64).sin();
        let next_r = 2.0 + pulse + 0.22 * (t * 2.1 + idx as f64 + 1.0).sin();
        let mid_a = (a + next) * 0.5;
        let mid_r = 1.55 + 0.25 * (t * 1.8 + idx as f64 * 0.9).cos();

        points.push(dvec3(r * a.cos(), r * a.sin(), 0.0));
        points.push(dvec3(mid_r * mid_a.cos(), mid_r * mid_a.sin(), 0.0));

        if idx == count - 1 {
            points.push(dvec3(next_r * next.cos(), next_r * next.sin(), 0.0));
        }
    }

    let mut item = VItem::from_vpoints(points);
    item.close();
    item.fill_rgbas = vec![
        Rgba(vec4(0.10, 0.38, 0.95, 0.88)),
        Rgba(vec4(0.96, 0.28, 0.42, 0.76)),
        Rgba(vec4(0.12, 0.82, 0.60, 0.72)),
    ]
    .into();
    item.stroke_rgbas = vec![
        Rgba(vec4(1.0, 1.0, 1.0, 1.0)),
        Rgba(vec4(0.96, 0.86, 0.32, 1.0)),
    ]
    .into();
    item.stroke_widths = vec![Width(0.035), Width(0.075), Width(0.035)].into();
    item
}
