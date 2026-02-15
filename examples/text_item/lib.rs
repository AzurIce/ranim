use std::f64::consts::TAU;

use ranim::{glam::DVec3, prelude::*};
use ranim_anims::{lagged::LaggedAnim, rotating::RotatingAnim, creation::WritingAnim};
use ranim_items::vitem::{VItem, text::TextItem};

#[scene]
#[output(dir = "text_item")]
fn text_item(r: &mut RanimScene) {
    let _r_cam = r.insert(CameraFrame::default());
    let text = "The quick brown fox jumps over the lazy dog.";

    let r_text = r.insert_empty();
    let tl = r.timeline_mut(r_text);
    let i_text = TextItem::new(text, 0.5).with(|item| item.move_to(DVec3::ZERO).discard());

    tl.play(Vec::<VItem>::from(i_text.clone()).lagged(0.1, |item| item.write()))
        .forward(1.)
        .play(
            i_text
                .clone()
                .rotating(TAU * 4., DVec3::Z)
                .with_duration(4.),
        )
        .forward(1.)
        .play(Vec::<VItem>::from(i_text.clone()).lagged(0.1, |item| item.unwrite()))
        .forward(1.);
}
