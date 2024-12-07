use std::time::{Duration, Instant};

use bevy_color::Srgba;
use env_logger::Env;
use log::info;
use ranim::glam::{vec2, Vec3};
use ranim::rabject::vmobject::TransformAnchor;
use ranim::rabject::Blueprint;
use ranim::{
    animation::{fading::Fading, transform::Transform},
    rabject::vmobject::{Arc, Polygon},
    scene::Scene,
};

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=info")).init();

    let mut scene = Scene::new();
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

    scene.wait(Duration::from_secs_f32(0.5));
    let polygon = scene.insert(polygon);
    scene.insert_updater(polygon, Fading::fade_in());
    scene.advance(Duration::from_secs_f32(1.0));
    scene.wait(Duration::from_secs_f32(0.5));

    let mut arc = Arc::new(std::f32::consts::PI / 2.0)
        .with_radius(100.0)
        .with_stroke_width(20.0)
        .build();
    arc.set_color(Srgba::hex("58C4DDFF").unwrap());

    let _src = scene.get(polygon).unwrap().clone();
    scene.insert_updater(polygon, Transform::new(_src, arc.clone()));
    scene.advance(Duration::from_secs_f32(1.0));
    scene.wait(Duration::from_secs_f32(0.5));

    scene.remove(polygon);
    let arc = scene.insert(arc);

    scene.insert_updater(arc, Fading::fade_out());
    scene.advance(Duration::from_secs_f32(1.0));

    // let arc = scene.play(Transform::new(polygon, arc)).unwrap();
    // scene.play(Fading::fade_out(arc));

    info!("Rendered {} frames in {:?}", scene.frame_count, t.elapsed());
}
