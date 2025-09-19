use itertools::Itertools;
use ranim::{
    anims::{fading::FadingAnim, lagged::LaggedAnim},
    color,
    color::HueDirection,
    glam::dvec2,
    items::vitem::{Group, geometry::Arc},
    prelude::*,
};

#[scene]
#[output(dir = "arc")]
pub fn arc(r: &mut RanimScene) {
    let _r_cam = r.insert_and_show(CameraFrame::default());

    // let frame_size = app.camera().size;
    let frame_size = dvec2(8.0 * 16.0 / 9.0, 8.0);
    let frame_start = dvec2(frame_size.x / -2.0, frame_size.y / -2.0);

    let start_color = color::color("#FF8080FF");
    let end_color = color::color("#58C4DDFF");

    let nrow = 10;
    let ncol = 10;
    let step_x = frame_size.x / ncol as f64;
    let step_y = frame_size.y / nrow as f64;

    let arcs = (0..nrow)
        .cartesian_product(0..ncol)
        .map(|(i, j)| {
            let (i, j) = (i as f64, j as f64);

            let angle = std::f64::consts::PI * (j + 1.0) / ncol as f64 * 360.0 / 180.0;
            let radius = step_y / 2.0 * 0.8;
            let color = start_color.lerp(
                end_color,
                i as f32 / (nrow - 1) as f32,
                HueDirection::Increasing,
            );
            let offset = frame_start + dvec2(j * step_x + step_x / 2.0, i * step_y + step_y / 2.0);
            Arc::new(angle, radius).with(|arc| {
                arc.stroke_width = 0.12 * (j as f32 + 0.02) / ncol as f32;
                arc.set_stroke_color(color)
                    .put_center_on(offset.extend(0.0));
            })
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
