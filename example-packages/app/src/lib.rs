use std::f64::consts::PI;

use ranim::{
    anims::{creation::WritingAnim, fading::FadingAnim, morph::MorphAnim},
    color::palettes::manim,
    items::vitem::{
        geometry::{Circle, Square},
        VItem,
    },
    prelude::*,
};

#[scene]
pub fn hello_ranim(r: &mut RanimScene) {
    let _r_cam = r.insert(CameraFrame::default());

    let mut square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });

    let r_square = r.insert_empty();
    {
        let timeline = r.timeline_mut(r_square);
        timeline.play(square.fade_in());
    };

    let circle = Circle::new(2.0).with(|circle| {
        circle
            .set_color(manim::RED_C)
            .with_origin(AabbPoint::CENTER, |x| {
                x.rotate_on_z(PI / 4.0 + PI);
            });
    });

    let mut vitem = VItem::from(square);
    r.timeline_mut(r_square)
        .play(vitem.morph_to(circle.into()))
        .forward(1.0)
        .play(vitem.clone().unwrite())
        .play(vitem.write())
        .play(vitem.fade_out());
}
