use std::time::{Duration, Instant};

use env_logger::Env;
use glam::{vec2, Vec3};
use log::info;
use ranim::prelude::*;
use ranim::color::palettes;
use ranim::rabject::vmobject::TransformAnchor;
use ranim::{
    animation::{fading::Fading, transform::Transform},
    rabject::vmobject::{Arc, Polygon},
    scene::SceneBuilder,
};

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("geometry_blueprints=trace"))
        .init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("geometry_blueprints=info"))
        .init();

    let mut scene = SceneBuilder::new("geometry_blueprints").build();
    let t = Instant::now();
    info!("running...");

    let mut polygon = Polygon::new(vec![
        vec2(-100.0, -300.0),
        vec2(0.0, 0.0),
        vec2(200.0, 300.0),
        vec2(0.0, 700.0),
        vec2(500.0, 0.0),
    ])
    .with_stroke_width(20.0)
    .build();
    polygon.set_color(palettes::manim::RED_C).rotate(
        std::f32::consts::PI / 4.0,
        Vec3::Z,
        TransformAnchor::origin(),
    );

    let polygon = scene.insert(polygon);
    scene.play(
        &polygon,
        Fading::fade_in().config(|config| {
            config.set_run_time(Duration::from_secs_f32(1.0));
        }),
    );

    let mut arc = Arc::new(std::f32::consts::PI / 2.0)
        .with_radius(100.0)
        .with_stroke_width(20.0)
        .build();
    arc.set_color(palettes::manim::BLUE_C);

    scene.play(&polygon, Transform::new(arc.clone()));

    let arc = scene.insert(arc);
    scene.play(&arc, Fading::fade_out());

    info!("Rendered {} frames in {:?}", scene.frame_count, t.elapsed());
}
