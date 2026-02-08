use ranim::glam;
use std::f64::consts::PI;

use glam::{DVec3, dvec2, dvec3};
use itertools::Itertools;
use ranim::{
    anims::{creation::WritingAnim, lagged::LaggedAnim, transform::TransformAnim},
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Polygon, Rectangle, Square},
        svg::SvgItem,
        typst::typst_svg,
    },
    prelude::*,
    utils::rate_functions::{linear, smooth},
};

fn build_logo(logo_width: f64) -> [VItem; 6] {
    let red_bg_rect = Rectangle::new(logo_width / 2.0, logo_width).with(|rect| {
        rect.set_color(manim::RED_C.with_alpha(0.5))
            .move_to(dvec3(-logo_width / 4.0, 0.0, 0.0));
    });
    let red_rect = Rectangle::new(logo_width / 4.0, logo_width).with(|rect| {
        rect.set_color(manim::RED_C).move_anchor_to(
            AabbPoint(dvec3(1.0, 0.0, 0.0)),
            dvec3(-logo_width / 4.0, 0.0, 0.0),
        );
    });

    let green_bg_sq = Square::new(logo_width / 2.0).with(|sq| {
        sq.set_color(manim::GREEN_C.with_alpha(0.5)).move_to(dvec3(
            logo_width / 4.0,
            logo_width / 4.0,
            0.0,
        ));
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
        sq.set_color(manim::BLUE_C.with_alpha(0.5)).move_to(dvec3(
            logo_width / 4.0,
            -logo_width / 4.0,
            0.0,
        ));
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
#[output(dir = "ranim_logo")]
fn ranim_logo(r: &mut RanimScene) {
    let _r_cam = r.insert(CameraFrame::default());
    let frame_size = dvec2(8.0 * 16.0 / 9.0, 8.0);
    let logo_width = frame_size.y * 0.618;

    let logo = build_logo(logo_width);
    let mut r_logo = logo.map(|item| (r.insert(item.clone()), item));

    let mut ranim_text = Vec::<VItem>::from(
        SvgItem::new(typst_svg(
            r#"
#align(center)[
    #text(10pt, font: "LXGW Bright")[Ranim]
]"#,
        ))
        .with(|text| {
            text.set_color(manim::WHITE)
                .scale_to(ScaleHint::PorportionalY(1.0))
                .move_to(DVec3::NEG_Y * 2.5);
        }),
    );
    let r_ranim_text = r.insert_empty();

    r_logo.iter_mut().for_each(|(r_logo, item)| {
        r.timeline_mut(*r_logo)
            .play(item.write().with_duration(3.0).with_rate_func(smooth));
    });
    r.timelines_mut().sync();

    let gap_ratio = 1.0 / 60.0;
    let gap = logo_width * gap_ratio;
    let scale = (logo_width - gap * 2.0) / logo_width;
    let scale = [
        dvec3(scale, 1.0, 1.0),
        dvec3(scale, scale, 1.0),
        dvec3(scale, scale, 1.0),
    ];
    let anchor = [
        AabbPoint(dvec3(-1.0, 0.0, 0.0)),
        AabbPoint(dvec3(1.0, 1.0, 0.0)),
        AabbPoint(dvec3(1.0, -1.0, 0.0)),
    ];
    r_logo
        .iter_mut()
        .chunks(2)
        .into_iter()
        .zip(scale.into_iter().zip(anchor))
        .for_each(|(chunk, (scale, anchor))| {
            chunk.for_each(|(r_item, item)| {
                r.timeline_mut(*r_item).play(
                    item.transform(|data| {
                        data.scale_at(scale, anchor)
                            .scale_at(dvec3(0.9, 0.9, 1.0), DVec3::ZERO)
                            .shift(dvec3(0.0, 1.3, 0.0));
                    })
                    .with_rate_func(smooth),
                );
            })
        });
    r.timeline_mut(r_ranim_text).forward(0.5).play(
        ranim_text
            .lagged(0.2, |item| {
                item.write().with_duration(2.0).with_rate_func(linear)
            })
            .with_duration(2.0),
    );
    r.timelines_mut().sync();

    r.insert_time_mark(
        r.timelines().max_total_secs(),
        TimeMark::Capture("preview.png".to_string()),
    );
    r.timelines_mut().forward(1.0);

    r_logo.iter_mut().for_each(|(r_logo_part, item)| {
        r.timeline_mut(*r_logo_part)
            .play(item.unwrite().with_duration(3.0).with_rate_func(smooth));
    });
    r.timeline_mut(r_ranim_text)
        .play(ranim_text.lagged(0.0, |item| {
            item.unwrite().with_duration(3.0).with_rate_func(linear)
        }));
}
