use std::f64::consts::PI;

use glam::{DVec3, dvec2, dvec3};
use ranim::{
    animation::{creation::WritingAnimSchedule, transform::GroupTransformAnimSchedule},
    color::palettes::manim,
    components::{Anchor, ScaleHint},
    items::{
        group::Group,
        vitem::{Polygon, Rectangle, Square, VItem},
    },
    prelude::*,
    timeline::TimeMark,
    typst_svg,
    utils::rate_functions::{linear, smooth},
};

fn build_logo(logo_width: f64) -> [VItem; 6] {
    let mut red_bg_rect = Rectangle(logo_width / 2.0, logo_width).build();
    red_bg_rect
        .set_color(manim::RED_C.with_alpha(0.5))
        .put_center_on(dvec3(-logo_width / 4.0, 0.0, 0.0));
    let mut red_rect = Rectangle(logo_width / 4.0, logo_width).build();
    red_rect
        .set_color(manim::RED_C)
        .put_anchor_on(Anchor::edge(1, 0, 0), dvec3(-logo_width / 4.0, 0.0, 0.0));

    let mut green_bg_sq = Square(logo_width / 2.0).build();
    green_bg_sq
        .set_color(manim::GREEN_C.with_alpha(0.5))
        .put_center_on(dvec3(logo_width / 4.0, logo_width / 4.0, 0.0));
    let mut green_triangle = Polygon(vec![
        dvec3(0.0, logo_width / 2.0, 0.0),
        dvec3(logo_width / 2.0, logo_width / 2.0, 0.0),
        dvec3(logo_width / 2.0, 0.0, 0.0),
    ])
    .build(); // ◥
    green_triangle.set_color(manim::GREEN_C);

    let mut blue_bg_sq = Square(logo_width / 2.0).build();
    blue_bg_sq
        .set_color(manim::BLUE_C.with_alpha(0.5))
        .put_center_on(dvec3(logo_width / 4.0, -logo_width / 4.0, 0.0));
    let mut blue_triangle = green_triangle.clone();
    blue_triangle
        .set_color(manim::BLUE_C)
        .rotate(PI, DVec3::Z)
        .shift(DVec3::NEG_Y * logo_width / 2.0); // ◣

    [
        red_bg_rect,
        red_rect,
        green_bg_sq,
        green_triangle,
        blue_bg_sq,
        blue_triangle,
    ]
}
#[scene]
struct RanimLogoScene;

impl TimelineConstructor for RanimLogoScene {
    fn construct<'t: 'r, 'r>(
        self,
        timeline: &'t RanimTimeline,
        _camera: &'r mut Rabject<'t, CameraFrame>,
    ) {
        let frame_size = dvec2(8.0 * 16.0 / 9.0, 8.0);
        let logo_width = frame_size.y * 0.618;

        let mut logo = build_logo(logo_width)
            .map(|item| timeline.insert(item))
            .into_iter()
            .collect::<Group<_>>();

        timeline
            .play(logo.lagged_anim(0.0, |item| {
                item.write().with_duration(3.0).with_rate_func(smooth)
            }))
            .sync();

        let gap_ratio = 1.0 / 60.0;
        let gap = logo_width * gap_ratio;
        let scale = (logo_width - gap * 2.0) / logo_width;
        let scale = [
            dvec3(scale, 1.0, 1.0),
            dvec3(scale, scale, 1.0),
            dvec3(scale, scale, 1.0),
        ];
        let anchor = [
            Anchor::edge(-1, 0, 0),
            Anchor::edge(1, 1, 0),
            Anchor::edge(1, -1, 0),
        ];
        logo.chunks_mut(2)
            .zip(scale.into_iter().zip(anchor))
            .for_each(|(chunk, (scale, anchor))| {
                timeline.play(
                    chunk
                        .transform(|data| {
                            data.scale_by_anchor(scale, anchor)
                                .scale_by_anchor(dvec3(0.9, 0.9, 1.0), Anchor::origin())
                                .shift(dvec3(0.0, 1.3, 0.0));
                        })
                        .with_rate_func(smooth)
                        .apply(),
                );
            });

        let mut ranim_text = Group::<VItem>::from_svg(typst_svg!(
            r#"
#align(center)[
    #text(10pt, font: "LXGW Bright")[Ranim]
]"#
        ));
        ranim_text
            .scale_to(ScaleHint::PorportionalHeight(1.0))
            .put_center_on(DVec3::NEG_Y * 2.5);
        let mut ranim_text = ranim_text
            .into_iter()
            .map(|item| timeline.insert(item))
            .collect::<Group<_>>();
        timeline.play(
            ranim_text
                .lagged_anim(0.2, |item| item.write())
                .with_duration(2.0)
                .with_rate_func(linear),
        );
        timeline.sync();

        timeline.insert_time_mark(
            timeline.duration_secs(),
            TimeMark::Capture("preview.png".to_string()),
        );
        timeline.forward(1.0);

        let mut all = logo.into_iter().chain(ranim_text).collect::<Group<_>>();
        timeline.play(all.lagged_anim(0.0, |item| {
            item.unwrite().with_duration(3.0).with_rate_func(smooth)
        }));
    }
}

fn main() {
    render_scene(RanimLogoScene, &AppOptions::default());
}
