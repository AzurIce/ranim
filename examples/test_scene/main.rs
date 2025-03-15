#![allow(clippy::all)]
#![allow(unused_imports)]
use std::{f32::consts::PI, time::Duration};

use env_logger::Env;
use glam::Vec3;
use ranim::{
    animation::{
        creation::{CreationAnim, WritingAnim},
        transform::{TransformAnim, TransformAnimSchedule},
    },
    components::Anchor,
    items::{
        camera_frame::CameraFrame,
        svg_item::SvgItem,
        vitem::{Square, VItem},
    },
    prelude::*,
    typst_svg,
};

// const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[scene]
struct TestScene;

impl TimelineConstructor for TestScene {
    fn construct<'t: 'r, 'r>(
        self,
        timeline: &'t RanimTimeline,
        camera: &'r mut Rabject<'t, CameraFrame>,
    ) {
        // let svg = SvgItem::from_svg(SVG);

        let svg = SvgItem::from_svg(typst_svg!("R"));

        let mut svg = timeline.insert(svg);
        let fill = svg.data.fill_color();
        svg.transform(|data| {
            data.scale(Vec3::splat(10.0))
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
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("test_scene=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("test_scene=info,ranim=trace"))
        .init();
    // println!("main");
    render_timeline(
        TestScene,
        &AppOptions {
            frame_rate: 60,
            ..AppOptions::default()
        },
    );
    // TestScene.render(&AppOptions {
    //     frame_rate: 60,
    //     frame_size: (3840, 2160),
    //     save_frames: true,
    //     ..Default::default()
    // });
}
