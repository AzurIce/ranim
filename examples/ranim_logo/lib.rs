use ranim::glam;
use std::f64::consts::PI;

use glam::{DVec3, dvec2, dvec3};
use itertools::Itertools;
use ranim::{
    anims::{creation::WritingAnim, morph::MorphAnim},
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Polygon, Rectangle, Square},
        svg::SvgItem,
        typst::typst_svg,
    },
    prelude::*,
    utils::rate_functions::smooth,
};

fn build_logo(logo_width: f64) -> [VItem; 6] {
    // The logo layout is intrinsic art coordinates, so the placement is
    // baked into point data right after converting to VItems.
    let red_bg_rect = VItem::from(Rectangle::new(logo_width / 2.0, logo_width)).with(|rect| {
        rect.set_color(manim::RED_C.with_alpha(0.5))
            .move_to(dvec3(-logo_width / 4.0, 0.0, 0.0));
    });
    let red_rect = VItem::from(Rectangle::new(logo_width / 4.0, logo_width)).with(|rect| {
        rect.set_color(manim::RED_C).move_anchor_to(
            AabbPoint(dvec3(1.0, 0.0, 0.0)),
            dvec3(-logo_width / 4.0, 0.0, 0.0),
        );
    });

    let green_bg_sq = VItem::from(Square::new(logo_width / 2.0)).with(|sq| {
        sq.set_color(manim::GREEN_C.with_alpha(0.5)).move_to(dvec3(
            logo_width / 4.0,
            logo_width / 4.0,
            0.0,
        ));
    });
    let green_triangle = VItem::from(Polygon::new(vec![
        dvec3(0.0, logo_width / 2.0, 0.0),
        dvec3(logo_width / 2.0, logo_width / 2.0, 0.0),
        dvec3(logo_width / 2.0, 0.0, 0.0),
    ]))
    .with(|tri| {
        tri.set_color(manim::GREEN_C);
    }); // ◥

    let blue_bg_sq = VItem::from(Square::new(logo_width / 2.0)).with(|sq| {
        sq.set_color(manim::BLUE_C.with_alpha(0.5)).move_to(dvec3(
            logo_width / 4.0,
            -logo_width / 4.0,
            0.0,
        ));
    });
    let blue_triangle = green_triangle.clone().with(|tri| {
        tri.set_color(manim::BLUE_C);
        tri.with_origin(AabbPoint::CENTER, |x| {
            x.rotate_on_z(PI);
        });
        tri.shift(DVec3::NEG_Y * logo_width / 2.0);
    }); // ◣

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
#[wasm_demo_doc]
#[output(dir = "./output/ranim_logo")]
fn ranim_logo(r: &mut RanimScene) {
    let frame_size = dvec2(8.0 * 16.0 / 9.0, 8.0);
    let logo_width = frame_size.y * 0.618;

    let logo = build_logo(logo_width);
    let mut logo_parts = logo.map(|mut item| {
        (
            seq![item.write().with_duration(3.0).with_rate_func(smooth)],
            item,
        )
    });

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
    logo_parts
        .iter_mut()
        .chunks(2)
        .into_iter()
        .zip(scale.into_iter().zip(anchor))
        .for_each(|(chunk, (scale, anchor))| {
            chunk.for_each(|(sequence, item)| {
                sequence.push(
                    item.morph(|data| {
                        data.with_origin(anchor, |x| {
                            x.scale(scale);
                        });
                        data.with_origin(DVec3::ZERO, |x| {
                            x.scale(dvec3(0.9, 0.9, 1.0));
                        });
                        data.shift(dvec3(0.0, 1.3, 0.0));
                    })
                    .with_rate_func(smooth),
                );
            })
        });
    let mut text_sequence = AnimSequence::new();
    text_sequence.forward(3.5).push(
        ranim_text
            .iter_mut()
            .map(|item| item.write())
            .into_lagged(0.2)
            .with_duration(2.0),
    );

    let phase_end = logo_parts
        .iter()
        .map(|(sequence, _)| sequence.cursor_sec())
        .chain(std::iter::once(text_sequence.cursor_sec()))
        .fold(0.0, f64::max);
    logo_parts
        .iter_mut()
        .for_each(|(sequence, _)| _ = sequence.hold_to(phase_end));
    text_sequence.hold_to(phase_end);

    r.insert_time_mark(phase_end, TimeMark::Capture("preview.png".to_string()));
    logo_parts
        .iter_mut()
        .for_each(|(sequence, _)| _ = sequence.hold(1.0));
    text_sequence.hold(1.0);

    logo_parts.iter_mut().for_each(|(sequence, item)| {
        sequence.push(item.unwrite().with_duration(3.0).with_rate_func(smooth));
    });
    text_sequence.push(
        ranim_text
            .iter_mut()
            .map(|item| item.unwrite())
            .into_lagged(0.0),
    );

    let total_secs = logo_parts
        .iter()
        .map(|(sequence, _)| sequence.cursor_sec())
        .chain(std::iter::once(text_sequence.cursor_sec()))
        .fold(0.0, f64::max);
    let mut content = AnimStack::new();
    for (sequence, _) in logo_parts {
        content.push(sequence);
    }
    r.play(CameraFrame::default().show().with_duration(total_secs));
    r.play(stack![content, text_sequence]);
}
