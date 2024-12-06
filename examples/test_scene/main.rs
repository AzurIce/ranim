use std::time::Instant;

use bevy_color::Alpha;
use env_logger::Env;
use glam::{vec3, Vec3};
use log::info;
use ranim::color::palettes;
// use ranim::animation::transform::Transform;
use ranim::glam::vec2;
use ranim::rabject::vgroup::VGroup;
use ranim::rabject::vmobject::TransformAnchor;
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
    let start = Instant::now();

    let mut polygon = Polygon::new(vec![
        vec2(-100.0, -300.0),
        vec2(500.0, 0.0),
        vec2(0.0, 700.0),
        vec2(200.0, 300.0),
        vec2(0.0, 0.0),
    ])
    .with_stroke_width(10.0)
    .build();
    polygon
        .rotate(
            std::f32::consts::PI / 4.0,
            Vec3::Z,
            TransformAnchor::origin(),
        )
        .set_color(palettes::manim::BLUE_C)
        .set_fill_color(palettes::manim::BLUE_C.with_alpha(0.5));
    // let polygon = scene.insert(polygon);
    // scene.render_to_image(&mut ctx, "output1.png");

    let mut arc = Arc::new(std::f32::consts::PI / 2.0)
        .with_radius(100.0)
        .with_stroke_width(20.0)
        .build();
    arc.set_color(palettes::manim::RED_C);
    arc.shift(vec3(-100.0, 100.0, 0.0));

    let group = scene.insert(VGroup::new(vec![arc, polygon]));

    // let mut transform = Transform::new(polygon.clone(), arc);

    // transform.func.interpolate(&mut polygon, 0.0);
    // scene.insert_rabject(&polygon);
    let t = Instant::now();
    scene.render_to_image("output-0.png");
    info!("[Main]: render to image cost {:?}", t.elapsed());

    // let t = Instant::now();
    // scene.get_mut(polygon).unwrap().set_color(palettes::manim::BLUE_C);
    // info!("[Main]: get mut and set color cost {:?}", t.elapsed());

    // let t = Instant::now();
    // scene.render_to_image("output-1.png");
    // info!("[Main]: render to image cost {:?}", t.elapsed());

    // transform.func.interpolate(&mut polygon, 0.5);
    // scene.insert_rabject(&polygon);
    // scene.render_to_image("output-0.5.png");

    info!(
        "Rendered {} frames in {:?}",
        scene.frame_count,
        start.elapsed()
    );
}
