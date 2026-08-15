use ranim::glam;

use glam::DVec3;
use ranim::{
    anims::{creation::WritingAnim, fading::FadingAnim},
    color::palettes::manim,
    core::animation::StaticAnim,
    items::vitem::{VItem, svg::SvgItem, typst::typst_svg},
    prelude::*,
    utils::rate_functions::smooth,
};

const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[scene]
#[wasm_demo_doc]
#[output(dir = "./output/basic")]
fn basic(r: &mut RanimScene) {
    let mut svg = Vec::<VItem>::from(SvgItem::new(SVG).with(|svg| {
        svg.scale_to_with_stroke(ScaleHint::PorportionalY(3.0))
            .move_to(DVec3::Y * 2.0);
    }));
    let mut text = Vec::<VItem>::from(
        SvgItem::new(typst_svg(
            r#"
            #align(center)[
                #text(18pt)[Ranim]

                #text(6pt)[Hello 你好]
            ]
            "#,
        ))
        .with(|text| {
            text.scale_to_with_stroke(ScaleHint::PorportionalY(2.0))
                .move_to(DVec3::NEG_Y * 2.0)
                .set_color(manim::WHITE)
                .set_fill_opacity(0.8);
        }),
    );

    let mut svg_sequence = AnimSequence::new();
    svg_sequence
        .hold(0.2)
        .push(svg.fade_in().with_duration(3.0).with_rate_func(smooth));

    let mut text_sequence = AnimSequence::new();
    text_sequence.hold(0.2).push(
        text.iter_mut()
            .map(|item| item.write().with_rate_func(smooth))
            .into_lagged(0.2)
            .with_duration(3.0),
    );

    let total_secs = svg_sequence.cursor_sec().max(text_sequence.cursor_sec());
    r.play(CameraFrame::default().show().with_duration(total_secs));
    r.play(stack![svg_sequence, text_sequence]);

    r.insert_time_mark(total_secs, TimeMark::Capture("preview.png".to_string()));
}
