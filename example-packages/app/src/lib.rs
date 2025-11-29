use std::f64::consts::PI;

use ranim::{
    anims::{creation::WritingAnim, fading::FadingAnim, transform::TransformAnim},
    color::palettes::manim,
    glam::DVec3,
    items::vitem::{
        VItem,
        geometry::{Circle, Square},
    },
    prelude::*,
};

#[scene]
pub fn hello_ranim(r: &mut RanimScene) {
    let _r_cam = r.insert(CameraFrame::default());

    let mut square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });

    let r_square = r.new_timeline();
    {
        let timeline = r.timeline_mut(r_square);
        timeline.play(square.fade_in());
    };

    let circle = Circle::new(2.0).with(|circle| {
        circle
            .set_color(manim::RED_C)
            .rotate(PI / 4.0 + PI, DVec3::Z);
    });

    let mut vitem = VItem::from(square);
    r.timeline_mut(r_square)
        .play(vitem.transform_to(circle.into()))
        .forward(1.0)
        .play(vitem.clone().unwrite())
        .play(vitem.write())
        .play(vitem.fade_out());
}
