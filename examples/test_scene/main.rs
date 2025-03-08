#![allow(clippy::all)]
#![allow(unused_imports)]
use std::{f32::consts::PI, time::Duration};

use env_logger::Env;
use glam::Vec3;
use ranim::{
    animation::{
        creation::{CreationAnim, WritingAnim},
        transform::TransformAnim,
    },
    components::TransformAnchor,
    items::{
        camera_frame::CameraFrame,
        svg_item::SvgItem,
        vitem::{Square, VItem},
        Rabject,
    },
    prelude::*,
    render_timeline,
    timeline::Timeline,
    typst_svg, AppOptions,
};
use ranim_macros::timeline;

// const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[timeline(fps = 60)]
fn test_scene(ranim: Ranim) {
    let Ranim(timeline, mut camera) = ranim;
    // let svg = SvgItem::from_svg(SVG);

    let svg = SvgItem::from_svg(typst_svg!("R"));

    let mut svg = timeline.insert(svg);
    let fill = svg.fill_color();
    svg.transform(|svg| {
        svg.scale(Vec3::splat(10.0))
            .set_fill_opacity(0.0)
            .set_stroke_width(1.0)
            .set_stroke_color(fill)
            .set_stroke_opacity(1.0);
    })
    .apply();
    // timeline.play(svg.unwrite().with_duration(2.0));
    // timeline.play(svg.uncreate().with_duration(2.0));
    timeline.play(camera.transform(|camera| camera.fovy = PI / 4.0));
    // timeline.play(svg.write().with_duration(2.0));
    // svg.transform(|svg| {
    //     svg.scale(Vec3::splat(3.272)).scale(Vec3::splat(2.0));
    // })
    // .apply();

    // timeline.forward(10.0);
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("test_scene=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("test_scene=info,ranim=trace"))
        .init();
    // println!("main");
    render_timeline!(test_scene);
    // TestScene.render(&AppOptions {
    //     frame_rate: 60,
    //     frame_size: (3840, 2160),
    //     save_frames: true,
    //     ..Default::default()
    // });
}
