use std::time::Instant;

use bevy_color::Srgba;
use env_logger::Env;
use log::info;
use ranim::animation::transform::Transform;
use ranim::glam::vec2;
use ranim::rabject::Blueprint;
use ranim::{
    rabject::vmobject::{Arc, Polygon},
    scene::Scene,
};

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("test_scene=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("test_scene=info,ranim=trace"))
        .init();

    let mut scene = Scene::new();
    let t = Instant::now();

    let mut polygon = Polygon::new(vec![
        vec2(-100.0, -300.0),
        vec2(500.0, 0.0),
        vec2(0.0, 700.0),
        vec2(200.0, 300.0),
        vec2(0.0, 0.0),
    ])
    .with_stroke_width(20.0)
    .build();
    scene.insert_rabject(&polygon);
    // scene.render_to_image(&mut ctx, "output1.png");

    let mut arc = Arc::new(std::f32::consts::PI / 2.0)
        .with_radius(100.0)
        .with_stroke_width(20.0)
        .build();
    arc.set_color(Srgba::hex("29ABCAFF").unwrap());

    let mut transform = Transform::new(polygon.clone(), arc);

    transform.func.interpolate(&mut polygon, 0.0);
    scene.insert_rabject(&polygon);
    scene.render_to_image("output-0.png");

    transform.func.interpolate(&mut polygon, 0.5);
    scene.insert_rabject(&polygon);
    scene.render_to_image("output-0.5.png");

    info!("Rendered {} frames in {:?}", scene.frame_count, t.elapsed());
}
