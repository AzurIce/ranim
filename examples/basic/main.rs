use std::time::{Duration, Instant};

use bevy_color::Srgba;
use env_logger::Env;
use log::info;
use ranim::glam::{vec2, Vec3};
use ranim::prelude::*;
use ranim::rabject::TransformAnchor;
use ranim::{
    animation::{fading::Fading, transform::Transform},
    rabject::rabject2d::blueprint::{Arc, Polygon},
    scene::world::WorldBuilder,
};

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=info,ranim=trace")).init();

    let mut world = WorldBuilder::new("basic").build();
    let t = Instant::now();
    info!("running...");

    let mut polygon = Polygon::new(vec![
        vec2(0.0, 0.0),
        vec2(-100.0, -300.0),
        vec2(500.0, 0.0),
        vec2(0.0, 700.0),
        vec2(200.0, 300.0),
    ])
    .with_stroke_width(10.0)
    .build();
    polygon.set_color(Srgba::hex("FF8080FF").unwrap()).rotate(
        std::f32::consts::PI / 4.0,
        Vec3::Z,
        TransformAnchor::origin(),
    );

    world.wait(Duration::from_secs_f32(0.5));
    let polygon = world.insert(polygon);

    info!("polygon fade_in");
    world.play(&polygon, Fading::fade_in());
    world.wait(Duration::from_secs_f32(0.5));

    let mut arc = Arc::new(std::f32::consts::PI / 2.0)
        .with_radius(100.0)
        .with_stroke_width(20.0)
        .build();
    arc.set_color(Srgba::hex("58C4DDFF").unwrap());

    info!("polygon transform to arc");
    world.play(&polygon, Transform::new(arc.clone()));
    world.wait(Duration::from_secs_f32(0.5));

    world.remove(polygon);
    let arc = world.insert(arc);

    info!("arc fade_out");
    world.play(&arc, Fading::fade_out());

    // let arc = scene.play(Transform::new(polygon, arc)).unwrap();
    // scene.play(Fading::fade_out(arc));

    info!("Rendered {} frames in {:?}", world.frame_count, t.elapsed());
}
