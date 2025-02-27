#![allow(clippy::all)]
#![allow(unused_imports)]
use std::time::Duration;

use env_logger::Env;
use glam::Vec3;
use ranim::{
    animation::{creation::CreationAnim, transform::TransformAnim},
    components::TransformAnchor,
    items::{
        svg_item::SvgItem,
        vitem::{Square, VItem},
        Rabject,
    },
    prelude::*,
    render_timeline,
    timeline::Timeline,
    AppOptions, TimelineConstructor,
};
use ranim_macros::timeline;

const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[timeline]
fn test_scene(timeline: &Timeline) {
    let svg = SvgItem::from_svg(SVG);

    let mut svg = timeline.insert(svg);
    svg.transform(|svg| {
        svg.scale(Vec3::splat(3.0));
    })
    .apply();

    timeline.forward(10.0);
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("test_scene=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("test_scene=info,ranim=trace"))
        .init();
    render_timeline!(test_scene);
    // TestScene.render(&AppOptions {
    //     frame_rate: 60,
    //     frame_size: (3840, 2160),
    //     save_frames: true,
    //     ..Default::default()
    // });
}
