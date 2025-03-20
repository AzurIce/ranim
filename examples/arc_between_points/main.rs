use env_logger::Env;
use itertools::Itertools;
use ranim::animation::fading::FadingAnimSchedule;
use ranim::color::HueDirection;
use ranim::glam::{Mat2, vec2};
use ranim::items::group::Group;
use ranim::items::vitem::ArcBetweenPoints;
use ranim::prelude::*;
use ranim::timeline::TimeMark;

#[scene]
struct ArcBetweenPointsScene;

impl TimelineConstructor for ArcBetweenPointsScene {
    fn construct<'t: 'r, 'r>(
        self,
        timeline: &'t RanimTimeline,
        _camera: &'r mut Rabject<'t, CameraFrame>,
    ) {
        let center = vec2(0.0, 0.0);

        let start_color = color!("#FF8080FF");
        let end_color = color!("#58C4DDFF");
        let ntan = 16;
        let nrad = 5;

        let arcs = (0..nrad)
            .map(|i| {
                let radius = 6.0 * (i + 1) as f32 / nrad as f32;
                let width = 6.0 * ((nrad - i) as f32 / nrad as f32).powi(2);
                let angle = std::f32::consts::PI * 7.0 / 4.0 * (i + 1) as f32 / nrad as f32;
                (radius, width, angle)
            })
            .cartesian_product(0..ntan)
            .map(|((rad, width, angle), j)| {
                let color = start_color.lerp(
                    end_color,
                    j as f32 / (ntan - 1) as f32,
                    HueDirection::Increasing,
                );
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
    env_logger::Builder::from_env(Env::default().default_filter_or("arc_between_points=trace"))
        .init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("arc_between_points=info"))
        .init();

    render_scene(ArcBetweenPointsScene, &AppOptions::default());
}
