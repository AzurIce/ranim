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
        vec2(-100.0, -300.0),
        vec2(0.0, 0.0),
        vec2(200.0, 300.0),
        vec2(0.0, 700.0),
        vec2(500.0, 0.0),
    ])
    .with_width(20.0)
    .build();
    polygon.set_color(Srgba::hex("FF8080FF").unwrap()).rotate(
        std::f32::consts::PI / 4.0,
        Vec3::Z,
        TransformAnchor::origin(),
    );

    let polygon = scene
        .play(Fading::fade_in(polygon).config(|config| {
            config.set_run_time(Duration::from_secs_f32(1.0));
        }))
        .unwrap();

    let mut arc = Arc::new(std::f32::consts::PI / 2.0)
        .with_radius(100.0)
        .with_stroke_width(20.0)
        .build();
    arc.set_color(Srgba::hex("58C4DDFF").unwrap());

    let arc = scene.play(Transform::new(polygon, arc)).unwrap();
    scene.play(Fading::fade_out(arc));

    info!("Rendered {} frames in {:?}", scene.frame_count, t.elapsed());
}
