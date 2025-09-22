#![allow(clippy::all)]
#![allow(unused)]
use ranim::glam;
use std::{f64::consts::PI, time::Duration};

use glam::{DVec3, dvec3};
use ranim::{
    anims::{
        creation::{CreationAnim, WritingAnim},
        fading::FadingAnim,
        transform::TransformAnim,
    },
    color::palettes::{
        css,
        manim::{self, BLUE_C, RED_C},
    },
    items::{
        Group,
        vitem::{
            self, VItem,
            geometry::{ArcBetweenPoints, Polygon, Rectangle, Square},
        },
    },
    prelude::*,
};

// const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[scene]
#[output(save_frames = true, dir = "output")]
fn test(r: &mut RanimScene) {
    let _r_cam = r.insert_and_show(CameraFrame::default());
    let n = 8;
    let arcs = (0..n)
        .map(|i: i32| {
            let angle = i as f64 / (n - 1) as f64 * PI * 2.0;
            ArcBetweenPoints::new(DVec3::ZERO, dvec3(angle.cos(), angle.sin(), 0.0), PI)
        })
        .collect::<Group<_>>();
    let r_arcs = r.insert_and_show(arcs);

    // text.set_stroke_color(manim::RED_C)
    //     .set_stroke_width(0.05)
    //     .set_fill_color(BLUE_C)
    //     .set_fill_opacity(0.5);
    // text.scale_to(ScaleHint::PorportionalHeight(8.0 * 0.8));
    // let mut text = timeline.insert(text);
    // let arrow = Arrow::new(-3.0 * DVec3::X, 3.0 * DVec3::Y);
    // let mut arrow = timeline.insert(arrow);

    // timeline.play(arrow.transform(|data| {
    //     data.set_color(RED_C);
    //     data.put_start_and_end_on(DVec3::NEG_Y, DVec3::Y);
    // }));
    r.timelines_mut().forward(1.0);
}
