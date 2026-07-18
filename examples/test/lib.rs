#![allow(clippy::all)]
#![allow(unused)]
use ranim::glam;
use std::{f64::consts::PI, time::Duration};

use glam::{DVec3, dvec3};
use ranim::{
    anims::{
        creation::{CreationAnim, WritingAnim},
        fading::FadingAnim,
        morph::MorphAnim,
    },
    color::palettes::{
        css,
        manim::{self, BLUE_C, RED_C},
    },
    items::vitem::{
        self, VItem,
        geometry::{ArcBetweenPoints, Polygon, Rectangle, Square},
    },
    prelude::*,
};

// const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[scene]
#[output(save_frames = true, dir = "./output/output")]
fn test(r: &mut RanimScene) {
    let camera_frame = CameraFrame::default().with(|x| {
        x.perspective_blend = 1.0;
        x.pos = DVec3::Z * 5.0;
    });
    let mut square = VItem::from(Square::new(4.0).with(|x| {
        x.set_color(manim::BLUE_C).set_fill_opacity(0.5);
    }));
    let mut content = AnimSequence::new();
    content
        .play(square.morph(|x| {
            x.with_origin(AabbPoint::CENTER, |x| {
                x.rotate_on_y(PI / 2.0);
            });
        }))
        .hold(1.0);

    // text.set_stroke_color(manim::RED_C)
    //     .set_stroke_width(0.05)
    //     .set_fill_color(BLUE_C)
    //     .set_fill_opacity(0.5);
    // text.scale_to(ScaleHint::PorportionalY(8.0 * 0.8));
    // let mut text = timeline.insert(text);
    // let arrow = Arrow::new(-3.0 * DVec3::X, 3.0 * DVec3::Y);
    // let mut arrow = timeline.insert(arrow);

    // timeline.play(arrow.morph(|data| {
    //     data.set_color(RED_C);
    //     data.put_start_and_end_on(DVec3::NEG_Y, DVec3::Y);
    // }));
    let mut camera = AnimSequence::new();
    camera
        .play(camera_frame.show())
        .hold_to(content.cursor_sec());
    r.play(stack![camera, content]);
}
