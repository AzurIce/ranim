use std::time::{Duration, Instant};

use bevy_color::Srgba;
use env_logger::Env;
use glam::{vec2, Mat2};
use log::info;
use ranim::animation::fading;
use ranim::prelude::*;
use ranim::rabject::rabject2d::vmobject::VMobject;
use ranim::scene::SceneBuilder;

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("arc_between_points=trace"))
        .init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("arc_between_points=info"))
        .init();

    let mut scene = SceneBuilder::new("arc_between_points").build();
    let canvas = scene.insert_new_canvas(1920, 1080);
    scene.center_canvas_in_frame(&canvas);
    let center = vec2(1920.0 / 2.0, 1080.0 / 2.0);

    let start_color = Srgba::hex("FF8080FF").unwrap();
    let end_color = Srgba::hex("58C4DDFF").unwrap();
    let ntan = 16;
    let nrad = 5;

    let rad_step = 200.0 / nrad as f32;
    let width_step = 50.0 / (nrad as f32).powi(2);
    let angle_step = std::f32::consts::PI * 7.0 / 4.0 / nrad as f32;

    let t = Instant::now();
    for i in 0..nrad {
        let t = Instant::now();
        let rad = rad_step * (i + 1) as f32;
        let width = width_step * ((nrad - i) as f32).powi(2);
        let angle = angle_step * (i + 1) as f32;

        for j in 0..ntan {
            let color = start_color.lerp(&end_color, j as f32 / (ntan - 1) as f32);
            let vec = Mat2::from_angle(std::f32::consts::PI * 2.0 / ntan as f32 * j as f32)
                * vec2(rad, 0.0);

            let mut arc =
                VMobject::blueprint_arc_between_points(center, center + vec, angle).build();
            arc.set_color(color)
                .set_fill_opacity(0.0)
                .set_stroke_width(width);

            let _arc = scene.play_in_canvas(
                &canvas,
                arc,
                fading::fade_in().config(|config| {
                    config.set_run_time(Duration::from_secs_f32(3.0 / (nrad * ntan) as f32));
                }),
            );
        }
        info!(
            "rad [{i}/{nrad}] angle: {angle} width: {width} rad: {rad} cost: {:?}",
            t.elapsed()
        );
    }

    info!(
        "Rendered {} frames({}s) in {:?}",
        scene.frame_count,
        scene.time,
        t.elapsed()
    );
}
