use std::f64::consts::TAU;

use ranim::{glam::DVec3, prelude::*, utils::rate_functions::ease_in_out_cubic};
use ranim_anims::rotating::RotatingAnim;
use ranim_core::animation::StaticAnim;
use ranim_items::vitem::text::TextItem;

#[scene]
#[output(dir = "text_item")]
fn text_item(r: &mut RanimScene) {
    let _r_cam = r.insert(CameraFrame::default());
    let text = "The quick brown fox jumps over the lazy dog.";

    let r_text = r.insert_empty();
    let tl = r.timeline_mut(r_text);
    let i_text = TextItem::new(text, 0.25).with(|item| item.move_to(DVec3::ZERO).discard());

    tl.play(i_text.show())
        .forward(1.)
        .play(
            i_text
                .clone()
                .rotating(TAU * 4., DVec3::Z)
                .with_duration(2.)
                .with_rate_func(ease_in_out_cubic),
        )
        .forward(2.);
}
