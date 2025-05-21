use itertools::Itertools;
use log::LevelFilter;
use ranim::{
    animation::{AnimGroupFunction, fading::FadingAnim},
    color::HueDirection,
    glam::{DMat2, dvec2},
    items::vitem::geometry::ArcBetweenPoints,
    prelude::*,
    timeline::TimeMark,
};

#[scene]
struct ArcBetweenPointsScene;

impl TimelineConstructor for ArcBetweenPointsScene {
    fn construct(self, timeline: &RanimTimeline, _camera: PinnedItem<CameraFrame>) {
        let center = dvec2(0.0, 0.0);

        let start_color = color!("#FF8080FF");
        let end_color = color!("#58C4DDFF");
        let ntan = 16;
        let nrad = 5;

        let arcs = (0..nrad)
            .map(|i| {
                let radius = 6.0 * (i + 1) as f64 / nrad as f64;
                let width = 0.12 * ((nrad - i) as f64 / nrad as f64).powi(2);
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
                ArcBetweenPoints::new(center.extend(0.0), (center + vec).extend(0.0), angle).with(
                    |arc| {
                        arc.stroke_width = width as f32;
                        arc.set_stroke_color(color);
                    },
                )
            })
            .collect::<Vec<_>>();

        let arcs_fade_in = arcs
            .into_iter()
            .map(|arc| arc.fade_in())
            .collect::<Vec<_>>()
            .with_lagged_offset(0.2)
            .with_epilogue_to_end()
            .with_total_duration(3.0);
        timeline.play(arcs_fade_in);

        timeline.insert_time_mark(
            timeline.cur_sec(),
            TimeMark::Capture("preview.png".to_string()),
        );
    }
}

fn main() {
    #[cfg(debug_assertions)]
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("ranim"), LevelFilter::Trace)
        .init();
    #[cfg(not(debug_assertions))]
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("ranim"), LevelFilter::Info)
        .init();

    render_scene(ArcBetweenPointsScene, &AppOptions::default());
}
