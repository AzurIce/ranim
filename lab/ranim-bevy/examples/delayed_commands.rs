mod common;

use std::f64::consts::TAU;

use bevy::prelude::*;
use ranim_bevy::RanimVItem;

fn main() {
    common::run_example(
        common::ExampleConfig::new("Ranim Bevy: Delayed Commands", "delayed_commands", 4.0),
        build_scene,
    );
}

fn build_scene(app: &mut App) {
    app.add_systems(Startup, (setup_scene, queue_vitems).chain())
        .add_systems(Update, animate_spawned_vitems);
}

#[derive(Component)]
struct DelayedVItem {
    phase: f32,
    radius: f32,
}

fn setup_scene(mut commands: Commands) {
    common::camera(
        &mut commands,
        Transform::from_xyz(0.0, 0.0, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    );
    common::light(&mut commands);
}

fn queue_vitems(mut commands: Commands) {
    let mut delayed = commands.delayed();

    for idx in 0..18 {
        let phase = idx as f32 / 18.0 * std::f32::consts::TAU;
        let radius = 2.6 + (idx % 3) as f32 * 0.34;
        let delay = idx as f32 * 0.045;
        let item = common::ring_segment(
            idx as f64 / 18.0 * TAU,
            (idx as f64 + 0.72) / 18.0 * TAU,
            radius as f64,
            0.42,
            palette(idx, 0.62),
            [1.0, 1.0, 1.0, 0.82],
        );

        let entity = delayed
            .secs(delay)
            .spawn((
                DelayedVItem { phase, radius },
                RanimVItem::new(item),
                Transform::from_xyz(0.0, 0.0, idx as f32 * 0.018),
            ))
            .id();

        delayed
            .secs(delay + 1.4)
            .entity(entity)
            .insert(Transform::from_xyz(0.0, 0.0, idx as f32 * 0.018)
                .with_scale(Vec3::splat(1.08)));
    }
}

fn animate_spawned_vitems(time: Res<Time>, mut query: Query<(&DelayedVItem, &mut RanimVItem)>) {
    let t = time.elapsed_secs();
    for (meta, mut vitem) in &mut query {
        let start = meta.phase as f64 + t as f64 * 0.45;
        let sweep = 0.55 + 0.18 * (t + meta.phase).sin() as f64;
        vitem.item = common::ring_segment(
            start,
            start + sweep,
            meta.radius as f64,
            0.34,
            palette((meta.phase * 40.0) as usize, 0.58),
            [1.0, 1.0, 1.0, 0.76],
        );
    }
}

fn palette(idx: usize, alpha: f32) -> [f32; 4] {
    const COLORS: [[f32; 3]; 6] = [
        [0.07, 0.47, 0.92],
        [0.88, 0.18, 0.34],
        [0.08, 0.72, 0.50],
        [0.96, 0.78, 0.18],
        [0.55, 0.30, 0.92],
        [0.95, 0.42, 0.18],
    ];
    let [r, g, b] = COLORS[idx % COLORS.len()];
    [r, g, b, alpha]
}
