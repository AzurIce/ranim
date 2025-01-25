use std::default;
use std::time::{Duration, Instant};

use bevy_color::{Alpha, Srgba};
use env_logger::Env;
use glam::vec2;
use log::info;
use ranim::animation::fading;
use ranim::items::vitem::{Arc, VItem};
// use ranim::rabject::rabject3d::RabjectEntity3d;
use ranim::{prelude::*, Scene};

struct ArcScene;

impl Scene for ArcScene {
    fn desc() -> ranim::SceneDesc {
        ranim::SceneDesc {
            name: "arc".to_string(),
        }
    }
    fn construct<T: ranim::RanimApp>(&mut self, app: &mut T) {
        let t = Instant::now();
        let frame_size = app.camera().size;
        let frame_start = vec2(frame_size.0 as f32 / -2.0, frame_size.1 as f32 / -2.0);

        let start_color = Srgba::hex("FF8080FF").unwrap();
        let end_color = Srgba::hex("58C4DDFF").unwrap();

        let nrow = 10;
        let ncol = 10;
        let gap = 10.0;
        let padding = 30.0;
        let step_x = (frame_size.0 as f32 - padding * 2.0 - gap * (ncol - 1) as f32) / ncol as f32;
        let step_y = (frame_size.1 as f32 - padding * 2.0 - gap * (nrow - 1) as f32) / nrow as f32;

        for i in 0..nrow {
            let t = Instant::now();
            for j in 0..ncol {
                let angle = std::f32::consts::PI * j as f32 / (ncol - 1) as f32 * 360.0 / 180.0;
                let radius = step_y / 2.0;
                let color = start_color.lerp(&end_color, i as f32 / (nrow - 1) as f32);
                let offset = frame_start + vec2(
                    j as f32 * step_x + step_x / 2.0 + j as f32 * gap + padding,
                    i as f32 * step_y + step_y / 2.0 + i as f32 * gap + padding,
                );
                let mut arc = Arc { angle, radius }.build();
                arc.stroke_widths.set_all(10.0 * j as f32);

                arc.stroke_rgbas.set_all(color);
                arc.fill_rgbas.set_all(color.with_alpha(0.0));
                arc.vpoints.shift(offset.extend(0.0));
                let arc = app.insert(arc);
                app.wait(Duration::from_secs_f32(0.02));
                // let _arc = scene.play_in_canvas(
                //     &canvas,
                //     arc,
                //     fading::fade_in().config(|config| {
                //         config.set_run_time(Duration::from_secs_f32(0.02));
                //     }),
                // );
            }
            info!("row [{i}/{nrow}] cost: {:?}", t.elapsed());
        }
        info!("total cost: {:?}", t.elapsed());

        // info!(
        //     "Rendered {} frames({}s) in {:?}",
        //     scene.frame_count,
        //     scene.time,
        //     t.elapsed()
        // );
    }
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("arc=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("arc=info")).init();

    ArcScene.render();
}
