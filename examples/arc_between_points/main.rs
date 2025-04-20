use env_logger::Env;
use itertools::Itertools;
use ranim::animation::fading::FadingAnimSchedule;
use ranim::color::HueDirection;
use ranim::glam::{DMat2, dvec2};
use ranim::items::group::Group;
use ranim::items::vitem::ArcBetweenPoints;
use ranim::prelude::*;
use ranim::timeline::TimeMark;

#[scene]
struct ArcBetweenPointsScene;

impl TimelineConstructor for ArcBetweenPointsScene {
    fn construct(
        self,
        timeline: &RanimTimeline,
        _camera: &mut Rabject<CameraFrame>,
    ) {
        let center = dvec2(0.0, 0.0);

        let start_color = color!("#FF8080FF");
        let end_color = color!("#58C4DDFF");
        let ntan = 16;
        let nrad = 5;

        let arcs = (0..nrad)
            .map(|i| {
                let radius = 6.0 * (i + 1) as f64 / nrad as f64;
                let width = 6.0 * ((nrad - i) as f64 / nrad as f64).powi(2);
                let angle = std::f64::consts::PI * 7.0 / 4.0 * (i + 1) as f64 / nrad as f64;
                (radius, width, angle)
            })
            .cartesian_product(0..ntan)
            .map(|((rad, width, angle), j)| {
                let color = start_color.lerp(
                    end_color,
                    j as f32 / (ntan - 1) as f32,
                    HueDirection::Increasing,
                );
                let vec = DMat2::from_angle(std::f64::consts::PI * 2.0 / ntan as f64 * j as f64)
                    * dvec2(rad, 0.0);
                let mut arc = ArcBetweenPoints {
                    start: center.extend(0.0),
                    end: (center + vec).extend(0.0),
                    angle,
                }
                .build();
                arc.set_color(color)
                    .set_fill_opacity(0.0)
                    .set_stroke_width(width as f32);
                arc
            })
            .collect::<Group<_>>();
        let mut arcs = timeline.insert(arcs);

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
