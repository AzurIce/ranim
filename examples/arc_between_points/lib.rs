use itertools::Itertools;
use ranim::{
    anims::{fading::FadingAnim, lagged::LaggedAnim},
    color,
    color::HueDirection,
    glam::{DMat2, dvec2},
    items::vitem::{Group, geometry::ArcBetweenPoints},
    prelude::*,
    timeline::TimeMark,
};

#[scene]
#[output(dir = "arc_between_points")]
fn arc_between_points(r: &mut RanimScene) {
    let _r_cam = r.insert_and_show(CameraFrame::default());
    let center = dvec2(0.0, 0.0);

    let start_color = color::color("#FF8080FF");
    let end_color = color::color("#58C4DDFF");
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
        .collect::<Group<_>>();
    let r_arcs = r.insert(arcs);

    r.timeline_mut(&r_arcs)
        .play_with(|arcs| arcs.lagged(0.2, |arc| arc.fade_in()).with_duration(3.0));

    r.insert_time_mark(
        r.timelines().max_total_secs(),
        TimeMark::Capture("preview.png".to_string()),
    );
}
