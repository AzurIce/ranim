mod common;

use bevy::prelude::*;
use ranim_bevy::RanimVItem;
use ranim_core::{VItem, traits::Interpolatable};

fn main() {
    common::run_example(
        common::ExampleConfig::new("Ranim Bevy: VItem Morph", "vitem_morph", 6.0),
        build_scene,
    );
}

fn build_scene(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, animate_shape);
}

#[derive(Component)]
struct MorphingShape {
    from: VItem,
    to: VItem,
}

fn setup(mut commands: Commands) {
    common::camera(
        &mut commands,
        Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    );
    common::light(&mut commands);

    let from = common::regular_polygon(
        8,
        1.9,
        [0.10, 0.36, 0.88, 0.86],
        [1.0, 1.0, 1.0, 0.95],
        0.04,
    );
    let to = common::star(
        4,
        2.35,
        0.78,
        std::f64::consts::FRAC_PI_4,
        [0.96, 0.27, 0.42, 0.82],
        [1.0, 0.92, 0.36, 1.0],
        0.055,
    );

    commands.spawn((
        MorphingShape {
            from: from.clone(),
            to,
        },
        RanimVItem::new(from),
        Transform::from_scale(Vec3::splat(0.9)),
    ));
}

fn animate_shape(time: Res<Time>, mut query: Query<(&MorphingShape, &mut RanimVItem)>) {
    let t = time.elapsed_secs_f64();
    let alpha = (t * 0.65).sin() * 0.5 + 0.5;

    for (shape, mut vitem) in &mut query {
        vitem.item = shape.from.lerp(&shape.to, alpha);
    }
}
