use env_logger::Env;
use glam::vec2;
use itertools::Itertools;
use ranim::animation::fading::FadingAnimSchedule;
use ranim::color::HueDirection;
use ranim::components::Anchor;
use ranim::items::group::Group;
use ranim::items::nvitem::Arc;
use ranim::prelude::*;
use ranim::timeline::TimeMark;

#[scene]
struct ArcScene;

impl TimelineConstructor for ArcScene {
    fn construct<'t: 'r, 'r>(
        self,
        timeline: &'t RanimTimeline,
        _camera: &'r mut Rabject<'t, CameraFrame>,
    ) {
        // let frame_size = app.camera().size;
        let frame_size = vec2(8.0 * 16.0 / 9.0, 8.0);
        let frame_start = vec2(frame_size.x / -2.0, frame_size.y / -2.0);

        let start_color = color!("#FF8080FF");
        let end_color = color!("#58C4DDFF");

        let nrow = 10;
        let ncol = 10;
        let step_x = frame_size.x / ncol as f32;
        let step_y = frame_size.y / nrow as f32;

        let arcs = (0..nrow)
            .cartesian_product(0..ncol)
            .map(|(i, j)| {
                let (i, j) = (i as f32, j as f32);

                let angle = std::f32::consts::PI * (j + 1.0) / ncol as f32 * 360.0 / 180.0;
                let radius = step_y / 2.0 * 0.8;
                let color =
                    start_color.lerp(end_color, i / (nrow - 1) as f32, HueDirection::Increasing);
                let offset =
                    frame_start + vec2(j * step_x + step_x / 2.0, i * step_y + step_y / 2.0);
                let mut arc = Arc { angle, radius }.build();
                arc.set_stroke_width(6.0 * (j + 1.0) / ncol as f32)
                    .set_stroke_color(color)
                    .set_fill_color(color.with_alpha(0.0))
                    .put_anchor_on(Anchor::center(), offset.extend(0.0));
                arc
            })
            .collect::<Group<_>>();

        let mut arcs = timeline.insert_group(arcs);
        let arcs_fade_in = arcs.lagged_anim(0.2, |item| item.fade_in());
        timeline.play(arcs_fade_in.with_total_duration(3.0)).sync();

        timeline.insert_time_mark(
            timeline.duration_secs(),
            TimeMark::Capture("preview.png".to_string()),
        );
    }
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("arc=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("arc=info")).init();

    render_scene(ArcScene, &AppOptions::default());
}
