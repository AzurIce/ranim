use bevy_color::{Alpha, Srgba};
use env_logger::Env;
use glam::vec2;
use ranim::animation::entity::fading::fade_in;
use ranim::animation::Timeline;
use ranim::items::vitem::Arc;
use ranim::{prelude::*, AppOptions, TimelineConstructor};

struct ArcScene;

impl TimelineConstructor for ArcScene {
    fn desc() -> ranim::SceneDesc {
        ranim::SceneDesc {
            name: "arc".to_string(),
        }
    }
    fn construct(&mut self, timeline: &mut Timeline) {
        // let frame_size = app.camera().size;
        let frame_size = (1920.0, 1080.0);
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
            for j in 0..ncol {
                let angle = std::f32::consts::PI * j as f32 / (ncol - 1) as f32 * 360.0 / 180.0;
                let radius = step_y / 2.0;
                let color = start_color.lerp(&end_color, i as f32 / (nrow - 1) as f32);
                let offset = frame_start
                    + vec2(
                        j as f32 * step_x + step_x / 2.0 + j as f32 * gap + padding,
                        i as f32 * step_y + step_y / 2.0 + i as f32 * gap + padding,
                    );
                let mut arc = Arc { angle, radius }.build();
                arc.set_stroke_width(10.0 * j as f32)
                    .set_stroke_color(color)
                    .set_fill_color(color.with_alpha(0.0))
                    .shift(offset.extend(0.0));

                let arc = timeline.insert(arc);
                timeline.play(fade_in(arc).with_duration(0.5));
            }
        }
    }
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("arc=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("arc=info")).init();

    ArcScene.render(&AppOptions::default());
}
