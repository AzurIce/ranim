use ranim::glam;

use glam::DVec3;
use ranim::{
    anims::{creation::WritingAnim, fading::FadingAnim, lagged::LaggedAnim},
    color::palettes::manim,
    items::vitem::{Group, VItem, svg::SvgItem, typst::typst_svg},
    prelude::*,
};

const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[scene]
#[output(dir = "basic")]
fn basic(r: &mut RanimScene) {
    let _r_cam = r.insert_and_show(CameraFrame::default());
    r.timelines_mut().forward(0.2);

    let svg = Group::<VItem>::from(SvgItem::new(SVG).with(|svg| {
        svg.scale_to_with_stroke(ScaleHint::PorportionalY(3.0))
            .put_center_on(DVec3::Y * 2.0);
    }));
    let text = Group::<VItem>::from(
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
                .put_center_on(DVec3::NEG_Y * 2.0)
                .set_color(manim::WHITE)
                .set_fill_opacity(0.8);
        }),
    );
    let r_svg = r.insert(svg);
    let r_text = r.insert(text);

    r.timeline_mut(&r_text)
        .play_with(|text| text.lagged(0.2, |e| e.write()).with_duration(3.0));
    r.timeline_mut(&r_svg)
        .play_with(|svg| svg.fade_in().with_duration(3.0)); // At the same time, the svg fade in
    r.timelines_mut().sync();

    r.insert_time_mark(
        r.timelines().max_total_secs(),
        TimeMark::Capture("preview.png".to_string()),
    );
}
