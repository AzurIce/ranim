use std::time::{Duration, Instant};

use bevy_color::Srgba;
use env_logger::Env;
use glam::vec2;
use log::info;
use ranim::animation::fading;
use ranim::prelude::*;
use ranim::rabject::rabject2d::vmobject::VMobject;
use ranim::scene::SceneBuilder;

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("arc=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("arc=info")).init();

    let mut scene = SceneBuilder::new("arc").build();
    let canvas = scene.insert_new_canvas(1920, 1080);
    scene.center_canvas_in_frame(&canvas);
    let t = Instant::now();

    let start_color = Srgba::hex("FF8080FF").unwrap();
    let end_color = Srgba::hex("58C4DDFF").unwrap();
    let nrow = 10;
    let ncol = 10;
    let gap = 10.0;
    let padding = 30.0;
    let step_x =
        (scene.camera.frame.size.0 as f32 - padding * 2.0 - gap * (ncol - 1) as f32) / ncol as f32;
    let step_y =
        (scene.camera.frame.size.1 as f32 - padding * 2.0 - gap * (nrow - 1) as f32) / nrow as f32;

    for i in 0..nrow {
        let t = Instant::now();
        for j in 0..ncol {
            let angle = std::f32::consts::PI * j as f32 / (ncol - 1) as f32 * 360.0 / 180.0;
            let color = start_color.lerp(&end_color, i as f32 / (nrow - 1) as f32);
            let offset = vec2(
                j as f32 * step_x + step_x / 2.0 + j as f32 * gap + padding,
                i as f32 * step_y + step_y / 2.0 + i as f32 * gap + padding,
            );
            let mut arc = VMobject::blueprint_arc(angle)
                .with_radius(step_y / 2.0)
                .with_stroke_width(10.0 * j as f32)
                .build();

            arc.set_color(color).set_fill_opacity(0.0).shift(offset);
            let _arc = scene.play_in_canvas(
                &canvas,
                arc,
                fading::fade_in().config(|config| {
                    config.set_run_time(Duration::from_secs_f32(0.02));
                }),
            );
        }
        info!("row [{i}/{nrow}] cost: {:?}", t.elapsed());
    }

    info!(
        "Rendered {} frames({}s) in {:?}",
        scene.frame_count,
        scene.time,
        t.elapsed()
    );
}
