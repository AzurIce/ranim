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
    content.push(square.fade_in());

    let circle = Circle::new(2.0).with(|circle| {
        circle
            .set_color(manim::RED_C)
            .with_origin(AabbPoint::CENTER, |x| {
                x.rotate_on_z(PI / 4.0 + PI);
            });
    });

    let mut vitem = VItem::from(square);
    content
        .push(vitem.morph_to(circle.into()))
        .hold(1.0)
        .push(vitem.clone().unwrite())
        .push(vitem.write())
        .push(vitem.fade_out());

    let mut camera = AnimSequence::new();
    camera
        .push(CameraFrame::default().show())
        .hold_to(content.cursor_sec());
    r.play(camera);
    r.play(content);
}
