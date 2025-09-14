use std::f64::consts::PI;

use ranim::{
    animation::{creation::WritingAnim, fading::FadingAnim, transform::TransformAnim},
    color::palettes::manim,
    glam::DVec3,
    items::vitem::{
        VItem,
        geometry::{Circle, Square},
    },
    prelude::*,
    timeline::TimeMark,
};

#[scene]
#[preview]
#[output(dir = "hello_ranim")]
fn hello_ranim(r: &mut RanimScene) {
    let _r_cam = r.insert_and_show(CameraFrame::default());
    let square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });

    let r_square = r.insert(square);
    {
        let timeline = r.timeline_mut(&r_square);
        timeline.play_with(|square| square.fade_in());
    };

    let circle = Circle::new(2.0).with(|circle| {
        circle
            .set_color(manim::RED_C)
            .rotate(-PI / 4.0 + PI, DVec3::Z);
    });

    let r_vitem = r.map(r_square, VItem::from);
    {
        let timeline = r.timeline_mut(&r_vitem);
        timeline.play_with(|state| state.transform_to(circle.into()));
        timeline.forward(1.0);
        let circle = timeline.state().clone();
        timeline.play_with(|circle| circle.unwrite());
        timeline.play(circle.write());
        timeline.play_with(|circle| circle.fade_out());
    };

    r.insert_time_mark(3.7, TimeMark::Capture("preview.png".to_string()));
}
