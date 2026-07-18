use std::f64::consts::PI;

use ranim::{
    anims::{creation::WritingAnim, fading::FadingAnim, morph::MorphAnim},
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Circle, Square},
    },
    prelude::*,
};

#[scene]
pub fn hello_ranim(r: &mut RanimScene) {
    let mut square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });

    let mut content = AnimSequence::new();
    content.play(square.fade_in());

    let circle = Circle::new(2.0).with(|circle| {
        circle
            .set_color(manim::RED_C)
            .with_origin(AabbPoint::CENTER, |x| {
                x.rotate_on_z(PI / 4.0 + PI);
            });
    });

    let mut vitem = VItem::from(square);
    content
        .play(vitem.morph_to(circle.into()))
        .hold(1.0)
        .play(vitem.clone().unwrite())
        .play(vitem.write())
        .play(vitem.fade_out());

    let mut camera = AnimSequence::new();
    camera
        .play(CameraFrame::default().show())
        .hold_to(content.cursor_sec());
    r.play(stack![camera, content]);
}
