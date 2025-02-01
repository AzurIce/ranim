use std::time::{Duration, Instant};

use bevy_color::Srgba;
use env_logger::Env;
use glam::{vec2, Mat2};
use log::info;
use ranim::animation::creation::Color;
use ranim::animation::fading;
use ranim::items::vitem::ArcBetweenPoints;
use ranim::{prelude::*, AnimationClipConstructor, SceneDesc};

pub struct ArcBetweenPointsScene;

impl AnimationClipConstructor for ArcBetweenPointsScene {
    fn desc() -> ranim::SceneDesc {
        SceneDesc {
            name: "arc_between_points".to_string(),
        }
    }
    fn construct<T: ranim::RanimApp>(&mut self, app: &mut T) {
        let (width, height) = (1920.0, 1080.0);
        let center = vec2(0.0, 0.0);

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
                let mut arc = ArcBetweenPoints {
                    start: center.extend(0.0),
                    end: (center + vec).extend(0.0),
                    angle,
                }
                .build();
                arc.set_color(color)
                    .set_fill_opacity(0.0)
                    .set_stroke_width(width);

                let _arc = app.play(
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
            app.frame_cnt(),
            app.frame_time(),
            t.elapsed()
        );
    }
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("arc_between_points=trace"))
        .init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("arc_between_points=info"))
        .init();

    ArcBetweenPointsScene.render();
}
