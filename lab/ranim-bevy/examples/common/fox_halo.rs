use std::f32::consts::{FRAC_PI_4, PI, TAU};

use bevy::{
    core_pipeline::oit::OrderIndependentTransparencySettings,
    light::CascadeShadowConfigBuilder,
    prelude::*,
    world_serialization::WorldInstanceReady,
};
use ranim_bevy::RanimVItem;

use crate::common;

pub const ASSET_PATH: &str = "assets";
const GLTF_PATH: &str = "models/animated/Fox.glb";

#[derive(Component)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}

#[derive(Component)]
pub(crate) struct OrbitCamera {
    radius: f32,
    height: f32,
    speed: f32,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct HaloSegment {
    layer: usize,
    segment: usize,
    radius: f32,
    thickness: f32,
    arc_count: usize,
    phase: f32,
    sweep: f32,
    spin: f32,
    pulse: f32,
    tilt: Vec3,
    height: f32,
    alpha: f32,
    color: [f32; 3],
}

pub fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(4.6, 2.5, 5.2).looking_at(Vec3::new(0.0, 0.45, 0.0), Vec3::Y),
        OrderIndependentTransparencySettings::default(),
        Msaa::Off,
        OrbitCamera {
            radius: 6.7,
            height: 2.45,
            speed: 0.16,
        },
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(24.0, 24.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.15, 0.12),
            perceptual_roughness: 0.92,
            ..default()
        })),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Torus::new(0.74, 0.78))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.18, 0.72, 0.96, 0.10),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.42, 0.0)
            .with_rotation(Quat::from_rotation_x(FRAC_PI_4))
            .with_scale(Vec3::new(1.08, 1.08, 0.16)),
    ));

    commands.spawn((
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.0)),
        DirectionalLight {
            illuminance: 22_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 5.0,
            maximum_distance: 18.0,
            ..default()
        }
        .build(),
    ));

    commands.spawn((
        PointLight {
            intensity: 950.0,
            range: 8.0,
            color: Color::srgb(0.45, 0.78, 1.0),
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(-2.2, 2.0, 2.7),
    ));
}

pub fn setup_fox(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let (graph, index) = AnimationGraph::from_clip(
        asset_server.load(GltfAssetLabel::Animation(2).from_asset(GLTF_PATH)),
    );

    commands
        .spawn((
            AnimationToPlay {
                graph_handle: graphs.add(graph),
                index,
            },
            WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(GLTF_PATH))),
            Transform::from_xyz(0.0, 0.0, 0.0)
                .with_rotation(Quat::from_rotation_y(PI))
                .with_scale(Vec3::splat(0.012)),
        ))
        .observe(play_animation_when_ready);
}

pub fn setup_halo(mut commands: Commands) {
    for layer in halo_layers() {
        for segment in 0..layer.arc_count {
            let halo = HaloSegment { segment, ..layer };
            commands.spawn((
                halo,
                RanimVItem::new(halo_item(halo, 0.0)),
                halo_transform(halo, 0.0),
            ));
        }
    }
}

fn play_animation_when_ready(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations_to_play: Query<&AnimationToPlay>,
    mut players: Query<&mut AnimationPlayer>,
) {
    let Ok(animation_to_play) = animations_to_play.get(scene_ready.entity) else {
        return;
    };

    for child in children.iter_descendants(scene_ready.entity) {
        if let Ok(mut player) = players.get_mut(child) {
            player.play(animation_to_play.index).repeat();
            commands
                .entity(child)
                .insert(AnimationGraphHandle(animation_to_play.graph_handle.clone()));
        }
    }
}

pub fn animate_halo_segments(
    time: Res<Time>,
    mut query: Query<(&HaloSegment, &mut RanimVItem, &mut Transform)>,
) {
    let t = time.elapsed_secs();

    for (halo, mut vitem, mut transform) in &mut query {
        vitem.item = halo_item(*halo, t);
        *transform = halo_transform(*halo, t);
    }
}

pub fn orbit_camera(time: Res<Time>, mut query: Query<(&OrbitCamera, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (camera, mut transform) in &mut query {
        let angle = t * camera.speed + 0.62;
        transform.translation = Vec3::new(
            angle.cos() * camera.radius,
            camera.height + 0.18 * (t * 0.7).sin(),
            angle.sin() * camera.radius,
        );
        transform.look_at(Vec3::new(0.0, 0.48, 0.0), Vec3::Y);
    }
}

fn halo_layers() -> Vec<HaloSegment> {
    vec![
        HaloSegment {
            layer: 0,
            segment: 0,
            radius: 0.70,
            thickness: 0.155,
            arc_count: 3,
            phase: 0.10,
            sweep: 1.08,
            spin: 0.86,
            pulse: 2.8,
            tilt: Vec3::new(0.30, 0.03, 0.11),
            height: 0.40,
            alpha: 0.44,
            color: [0.24, 0.78, 1.0],
        },
        HaloSegment {
            layer: 1,
            segment: 0,
            radius: 0.88,
            thickness: 0.135,
            arc_count: 3,
            phase: 1.26,
            sweep: 0.95,
            spin: -0.58,
            pulse: 2.1,
            tilt: Vec3::new(-0.22, 0.20, 0.18),
            height: 0.58,
            alpha: 0.38,
            color: [1.0, 0.55, 0.20],
        },
        HaloSegment {
            layer: 2,
            segment: 0,
            radius: 1.06,
            thickness: 0.115,
            arc_count: 4,
            phase: 2.35,
            sweep: 0.82,
            spin: 0.42,
            pulse: 2.6,
            tilt: Vec3::new(0.10, -0.24, 0.30),
            height: 0.76,
            alpha: 0.34,
            color: [0.52, 0.92, 0.42],
        },
        HaloSegment {
            layer: 3,
            segment: 0,
            radius: 1.25,
            thickness: 0.096,
            arc_count: 4,
            phase: 3.10,
            sweep: 0.70,
            spin: -0.74,
            pulse: 1.9,
            tilt: Vec3::new(0.32, 0.12, -0.20),
            height: 0.94,
            alpha: 0.29,
            color: [0.96, 0.34, 0.52],
        },
        HaloSegment {
            layer: 4,
            segment: 0,
            radius: 1.46,
            thickness: 0.078,
            arc_count: 5,
            phase: 4.20,
            sweep: 0.58,
            spin: 0.31,
            pulse: 3.2,
            tilt: Vec3::new(-0.26, 0.10, -0.24),
            height: 1.10,
            alpha: 0.24,
            color: [0.78, 0.48, 1.0],
        },
    ]
}

fn halo_transform(halo: HaloSegment, time: f32) -> Transform {
    let layer_phase = halo.phase + halo.layer as f32 * 0.37;
    let bob = 0.035 * (time * halo.pulse + layer_phase).sin();
    Transform::from_xyz(0.0, halo.height + bob, 0.0).with_rotation(
        Quat::from_euler(EulerRot::XYZ, halo.tilt.x, halo.tilt.y, halo.tilt.z)
            * Quat::from_rotation_z(time * halo.spin * 0.34 + layer_phase),
    )
}

fn halo_item(halo: HaloSegment, time: f32) -> ranim_core::VItem {
    let wobble = 1.0 + 0.025 * (time * halo.pulse + halo.phase + halo.segment as f32).sin();
    let offset = TAU * halo.segment as f32 / halo.arc_count as f32;
    let center = halo.phase + offset + time * halo.spin;
    let sweep = halo.sweep * wobble;
    let item = common::ring_segment_with_stroke_width(
        (center - sweep * 0.5) as f64,
        (center + sweep * 0.5) as f64,
        (halo.radius * wobble) as f64,
        halo.thickness as f64,
        [
            halo.color[0],
            halo.color[1],
            halo.color[2],
            halo.alpha,
        ],
        [1.0, 1.0, 1.0, (halo.alpha + 0.10).min(0.46)],
        0.006,
    );

    item.with_normal(ranim_core::glam::DVec3::Z)
}
