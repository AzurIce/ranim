use std::time::{Duration, Instant};

use bevy_color::Srgba;
use env_logger::Env;
use glam::{Mat3, Vec3};
use log::info;
use ranim::glam::vec3;
use ranim::interpolate::Interpolatable;
use ranim::rabject::vmobject::ArcBetweenPoints;
use ranim::rabject::Blueprint;
use ranim::{animation::fading::Fading, scene::Scene};

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("arc_between_points=trace"))
        .init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("arc_between_points=info"))
        .init();

    let mut scene = Scene::new();

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
            let end = Mat3::from_rotation_z(std::f32::consts::PI * 2.0 / ntan as f32 * j as f32)
                * vec3(rad, 0.0, 0.0);

            let mut arc = ArcBetweenPoints::new(Vec3::ZERO, end, angle)
                .with_stroke_width(width)
                .build();
            arc.set_color(color);

            let arc = scene.insert(arc);
            scene.play(
                arc,
                Fading::fade_in().config(|config| {
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
