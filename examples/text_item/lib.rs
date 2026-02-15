use std::f64::consts::TAU;

use ranim::{color::palettes::manim, glam::DVec3, prelude::*};
use ranim_anims::{
    creation::{CreationAnim, WritingAnim},
    lagged::LaggedAnim,
    rotating::RotatingAnim,
};
use ranim_items::vitem::{VItem, text::TextItem};

#[scene]
#[output(dir = "text_item")]
fn text_item(r: &mut RanimScene) {
    let _r_cam = r.insert(CameraFrame::default());
    let text = "The quick brown fox jumps over the lazy dog.";

    let i_text = TextItem::new(text, 0.5).with(|item| item.move_to(DVec3::ZERO).discard());
    let i_text_box = i_text
        .text_box()
        .with(|item| item.set_stroke_color(manim::RED_C).discard());

    let r_text = r.insert_empty();
    let tl = r.timeline_mut(r_text);
    tl.play(
        Vec::<VItem>::from(i_text.clone())
            .lagged(0.1, |item| item.write())
            .with_duration(1.),
    )
    .forward(3.)
    .play(
        i_text
            .clone()
            .rotating(TAU * 4., DVec3::Z)
            .with_duration(4.),
    )
    .forward(1.)
    .play(Vec::<VItem>::from(i_text.clone()).lagged(0.1, |item| item.unwrite()))
    .forward(1.);

    let r_outline = r.insert_empty();
    let tl = r.timeline_mut(r_outline);
    tl.forward(1.)
        .play(VItem::from(i_text_box.clone()).create().with_duration(1.))
        .play(VItem::from(i_text_box.clone()).uncreate().with_duration(1.));
}
