use std::time::{Duration, Instant};

use bevy_color::Alpha;
use env_logger::Env;
use glam::{vec3, Vec3};
use log::info;
use ranim::animation::fading::Fading;
use ranim::animation::transform::Transform;
use ranim::color::palettes;
// use ranim::animation::transform::Transform;
use ranim::glam::vec2;
use ranim::prelude::*;
use ranim::rabject::svg_mobject::SvgMobject;
use ranim::rabject::vgroup::VGroup;
use ranim::rabject::vmobject::{Arc, Polygon};
use ranim::rabject::vmobject::{Circle, Dot, Ellipse, Square, TransformAnchor};
use ranim::rabject::vpath::{VPath, VPathPoint};
use ranim::scene::SceneBuilder;

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("test_scene=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("test_scene=info,ranim=trace")).init();

    let mut scene = SceneBuilder::new("test_scene").build();
    let start = Instant::now();

    // let line = VPath::line(Vec3::ZERO, vec3(100.0, 100.0, 0.0));
    // let vpath = scene.insert(line);

    // let quad = VPath::quad(Vec3::ZERO, vec3(100.0, 100.0, 0.0), vec3(200.0, 0.0, 0.0));
    // let vpath2 = scene.insert(quad);

    // let cubic = VPath::cubic(
    //     Vec3::ZERO,
    //     vec3(100.0, 100.0, 0.0),
    //     vec3(200.0, 100.0, 0.0),
    //     vec3(300.0, 0.0, 0.0),
    // );
    // let vpath3 = scene.insert(cubic);

    // let offset = vec3(100.0, 100.0, 0.0);
    // let offset = Vec3::ZERO;
    // let points = vec![
    //     VPathPoint::new(
    //         Vec3::ZERO + offset,
    //         Some(vec3(10.0, 100.0, 0.0) + offset),
    //         Some(vec3(10.0, -100.0, 0.0) + offset),
    //     ),
    //     VPathPoint::new(
    //         vec3(300.0, 0.0, 0.0) + offset,
    //         Some(vec3(200.0, -100.0, 0.0) + offset),
    //         Some(vec3(200.0, 100.0, 0.0) + offset),
    //     ),
    //     VPathPoint::new(
    //         Vec3::ZERO + offset,
    //         Some(vec3(10.0, 100.0, 0.0) + offset),
    //         Some(vec3(10.0, -100.0, 0.0) + offset),
    //     ),
    // ];

    // let close = VPath { points };

    // let vpath4 = scene.insert(close);

    // scene.render_to_image("vpath.png");

    let svg = SvgMobject::from_path("assets/test.svg");
    let svg = scene.insert(svg);
    scene.render_to_image("svg.png");

    // let mut polygon = Polygon::new(vec![
    //     vec2(-100.0, -300.0),
    //     vec2(500.0, 0.0),
    //     vec2(0.0, 700.0),
    //     vec2(200.0, 300.0),
    //     vec2(0.0, 0.0),
    // ])
    // .with_stroke_width(10.0)
    // .build();
    // polygon
    //     .rotate(
    //         std::f32::consts::PI / 4.0,
    //         Vec3::Z,
    //         TransformAnchor::origin(),
    //     )
    //     .set_color(palettes::manim::BLUE_C)
    //     .set_fill_color(palettes::manim::BLUE_C.with_alpha(0.5));

    // let mut arc = Arc::new(std::f32::consts::PI / 2.0)
    //     .with_radius(100.0)
    //     .with_stroke_width(20.0)
    //     .build();
    // arc.set_color(palettes::manim::RED_C);
    // arc.shift(vec3(-100.0, 100.0, 1.0));

    // let arc = scene.insert(arc);
    // let vgroup1 = scene.insert(VGroup::new(vec![arc, polygon]));
    // scene.play(&vgroup1, Fading::fade_in());

    // let mut circle = Circle::new(100.0).build();
    // circle.shift(vec3(-100.0, 0.0, 0.0));
    // let mut square = Square::new(100.0).build();
    // square.shift(vec3(100.0, 0.0, 0.0));
    // let vgroup2 = VGroup::new(vec![circle, square]);

    // scene.play(&vgroup1, Transform::new(vgroup2.clone()));
    // scene.remove(vgroup1);
    // let vgroup2 = scene.insert(vgroup2);

    // scene.wait(Duration::from_secs_f32(0.5));

    // let mut ellipse = Ellipse::new(100.0, 200.0).build();
    // ellipse
    //     .set_color(palettes::manim::YELLOW_B.with_alpha(0.5))
    //     .set_stroke_color(palettes::manim::YELLOW_B);

    // let mut dot = Dot::new(vec3(0.0, -100.0, 0.0)).build();
    // dot.set_color(palettes::manim::GREEN_C);

    // let vgroup3 = VGroup::new(vec![dot, ellipse]);
    // scene.play_remove(vgroup2, Transform::new(vgroup3.clone()));

    // let vgroup3 = scene.insert(vgroup3);
    // scene.play_remove(vgroup3, Fading::fade_out());

    info!(
        "Rendered {} frames in {:?}",
        scene.frame_count,
        start.elapsed()
    );
}
