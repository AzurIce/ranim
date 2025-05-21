use std::f64::consts::PI;

use glam::{DVec3, dvec2, dvec3};
use itertools::Itertools;
use ranim::{
    animation::{AnimGroupFunction, creation::WritingAnim, transform::GroupTransformAnim},
    color::palettes::manim,
    components::{Anchor, ScaleHint},
    items::{
        group::Group,
        vitem::{
            VItem,
            geometry::{Polygon, Rectangle, Square},
        },
    },
    prelude::*,
    timeline::TimeMark,
    typst_svg,
    utils::rate_functions::{linear, smooth},
};

fn build_logo(logo_width: f64) -> [VItem; 6] {
    let red_bg_rect = Rectangle::new(logo_width / 2.0, logo_width).with(|rect| {
        rect.set_color(manim::RED_C.with_alpha(0.5))
            .put_center_on(dvec3(-logo_width / 4.0, 0.0, 0.0));
    });
    let red_rect = Rectangle::new(logo_width / 4.0, logo_width).with(|rect| {
        rect.set_color(manim::RED_C)
            .put_anchor_on(Anchor::edge(1, 0, 0), dvec3(-logo_width / 4.0, 0.0, 0.0));
    });

    let green_bg_sq = Square::new(logo_width / 2.0).with(|sq| {
        sq.set_color(manim::GREEN_C.with_alpha(0.5))
            .put_center_on(dvec3(logo_width / 4.0, logo_width / 4.0, 0.0));
    });
    let green_triangle = Polygon::new(vec![
        dvec3(0.0, logo_width / 2.0, 0.0),
        dvec3(logo_width / 2.0, logo_width / 2.0, 0.0),
        dvec3(logo_width / 2.0, 0.0, 0.0),
    ])
    .with(|tri| {
        tri.set_color(manim::GREEN_C);
    }); // ◥

    let blue_bg_sq = Square::new(logo_width / 2.0).with(|sq| {
        sq.set_color(manim::BLUE_C.with_alpha(0.5))
            .put_center_on(dvec3(logo_width / 4.0, -logo_width / 4.0, 0.0));
    });
    let blue_triangle = green_triangle.clone().with(|tri| {
        tri.set_color(manim::BLUE_C)
            .rotate(PI, DVec3::Z)
            .shift(DVec3::NEG_Y * logo_width / 2.0);
    }); // ◣

    [
        VItem::from(red_bg_rect),
        VItem::from(red_rect),
        VItem::from(green_bg_sq),
        VItem::from(green_triangle),
        VItem::from(blue_bg_sq),
        VItem::from(blue_triangle),
    ]
}
#[scene]
struct RanimLogoScene;

impl TimelineConstructor for RanimLogoScene {
    fn construct(self, timeline: &RanimTimeline, _camera: PinnedItem<CameraFrame>) {
        let frame_size = dvec2(8.0 * 16.0 / 9.0, 8.0);
        let logo_width = frame_size.y * 0.618;

        let logo = build_logo(logo_width);
        let logo =
            timeline.play(logo.map(|item| item.write().with_duration(3.0).with_rate_func(smooth)));

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
        let logo_transform_anim = logo
            .into_iter()
            .chunks(2)
            .into_iter()
            .zip(scale.into_iter().zip(anchor))
            .flat_map(|(chunk, (scale, anchor))| {
                let chunk = chunk.collect_array::<2>().unwrap();
                chunk
                    .transform(|data| {
                        data.scale_by_anchor(scale, anchor)
                            .scale_by_anchor(dvec3(0.9, 0.9, 1.0), Anchor::ORIGIN)
                            .shift(dvec3(0.0, 1.3, 0.0));
                    })
                    .with_rate_func(smooth)
            })
            .collect_array::<6>()
            .unwrap();

        let ranim_text = Group::<VItem>::from_svg(typst_svg!(
            r#"
#align(center)[
    #text(10pt, font: "LXGW Bright")[Ranim]
]"#
        ))
        .with(|text| {
            text.set_color(manim::WHITE)
                .scale_to(ScaleHint::PorportionalY(1.0))
                .put_center_on(DVec3::NEG_Y * 2.5);
        });
        let (logo, end_time_logo) = timeline.schedule(logo_transform_anim);
        let (ranim_text, end_time_text) = timeline.schedule(
            ranim_text
                .into_iter()
                .map(|item| item.write().with_duration(2.0).with_rate_func(linear))
                .collect::<Vec<_>>()
                .with_lagged_offset(0.2)
                .with_epilogue_to_end(),
        );
        timeline.forward_to(end_time_logo);
        let logo = timeline.pin(logo);
        timeline.forward_to(end_time_text);
        let ranim_text = timeline.pin(ranim_text);

        timeline.insert_time_mark(
            timeline.cur_sec(),
            TimeMark::Capture("preview.png".to_string()),
        );
        timeline.forward(1.0);

        timeline.play(
            timeline
                .unpin(logo)
                .into_iter()
                .chain(timeline.unpin(ranim_text))
                .map(|item| item.unwrite().with_duration(3.0).with_rate_func(smooth))
                .collect::<Vec<_>>(),
        );
    }
}

fn main() {
    #[cfg(feature = "app")]
    run_scene_app(RanimLogoScene);
    #[cfg(not(feature = "app"))]
    render_scene(RanimLogoScene, &AppOptions::default());
}
