use std::time::{Duration, Instant};

use bevy_color::Srgba;
use env_logger::Env;
use log::info;
use ranim::animation::fading;
use ranim::glam::vec2;
use ranim::rabject::rabject2d::vmobject::{
    geometry::{Arc, Polygon},
    svg::Svg,
};
use ranim::{animation::transform::Transform, scene::SceneBuilder};
use ranim::{prelude::*, typst_svg};

const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=info")).init();

    let mut scene = SceneBuilder::new("basic").build();
    let canvas = scene.insert_new_canvas(1920, 1080);
    let center = vec2(1920.0 / 2.0, 1080.0 / 2.0);
    scene.center_canvas_in_frame(&canvas);

    let t = Instant::now();
    info!("running...");

    let mut ranim_text = Svg::from_svg(&typst_svg!("#text(60pt)[Ranim]")).build();
    ranim_text.shift(center - ranim_text.bounding_box().center());

    // 0.5s wait -> fade in -> 0.5s wait
    scene.wait(Duration::from_secs_f32(0.5));
    let ranim_text = scene.play_in_canvas(&canvas, ranim_text, fading::fade_in());
    scene.wait(Duration::from_secs_f32(0.5));

    let mut polygon = Polygon::new(vec![
        vec2(0.0, 0.0),
        vec2(-100.0, -300.0),
        vec2(500.0, 0.0),
        vec2(0.0, 700.0),
        vec2(200.0, 300.0),
    ])
    .with_stroke_width(10.0)
    .build();
    polygon
        .set_color(Srgba::hex("FF8080FF").unwrap())
        .set_opacity(0.5)
        .rotate(std::f32::consts::PI / 4.0)
        .shift(center);

    // 0.5s wait -> fade in -> 0.5s wait
    scene.wait(Duration::from_secs_f32(0.5));
    let polygon = scene.play_in_canvas(&canvas, ranim_text, Transform::new(polygon));
    scene.wait(Duration::from_secs_f32(0.5));

    let mut svg = Svg::from_svg(SVG).build();
    svg.shift(center);

    info!("polygon transform to svg");
    let svg = scene.play_in_canvas(&canvas, polygon, Transform::new(svg.clone()));
    scene.wait(Duration::from_secs_f32(0.5));

    let mut arc = Arc::new(std::f32::consts::PI / 2.0)
        .with_radius(100.0)
        .with_stroke_width(20.0)
        .build();
    arc.set_color(Srgba::hex("58C4DDFF").unwrap()).shift(center);

    info!("svg transform to arc");
    let arc = scene.play_in_canvas(&canvas, svg, Transform::new(arc.clone()));
    scene.wait(Duration::from_secs_f32(0.5));

    info!("arc fade_out");
    scene.play_remove_in_canvas(&canvas, arc, fading::fade_out());

    info!(
        "Rendered {} frames({}s) in {:?}",
        scene.frame_count,
        scene.time,
        t.elapsed()
    );
}
