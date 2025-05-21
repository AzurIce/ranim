#![allow(clippy::all)]
#![allow(unused)]
use std::{f64::consts::PI, time::Duration};

use ::color::palette::css;
use glam::{DVec3, dvec3};
use log::LevelFilter;
use ranim::{
    animation::{
        creation::{CreationAnim, WritingAnim},
        fading::FadingAnim,
        transform::TransformAnim,
    },
    color::palettes::manim::{self, BLUE_C, RED_C},
    components::{Anchor, ScaleHint},
    items::{
        camera_frame::CameraFrame,
        vitem::{
            self, VItem,
            geometry::{ArcBetweenPoints, Polygon, Rectangle, Square},
        },
    },
    prelude::*,
    typst_svg,
};

// const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[scene]
struct TestScene;

impl TimelineConstructor for TestScene {
    fn construct(self, timeline: &RanimTimeline, _camera: PinnedItem<CameraFrame>) {
        let n = 8;
        let arcs = (0..n)
            .map(|i| {
                let angle = i as f64 / (n - 1) as f64 * PI * 2.0;
                ArcBetweenPoints::new(DVec3::ZERO, dvec3(angle.cos(), angle.sin(), 0.0), PI)
            })
            .collect::<Vec<_>>();
        let arcs = timeline.pin(arcs);

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
        timeline.forward(1.0);
    }
}

fn main() {
    #[cfg(debug_assertions)]
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("ranim"), LevelFilter::Trace)
        .init();
    #[cfg(not(debug_assertions))]
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("ranim"), LevelFilter::Info)
        .init();
    // println!("main");
    // render_scene(
    //     TestScene,
    //     &AppOptions {
    //         frame_rate: 60,
    //         ..AppOptions::default()
    //     },
    // );
    // #[cfg(not(feature = "app"))]
    // render_scene_at_sec(
    //     TestScene,
    //     0.0,
    //     "test.png",
    //     &AppOptions {
    //         pixel_size: (1080, 1080),
    //         ..Default::default()
    //     },
    // );
    render_scene(TestScene, &AppOptions::default());

    // reuires "app" feature
    #[cfg(feature = "app")]
    run_scene_app(TestScene);
    // TestScene.render(&AppOptions {
    //     frame_rate: 60,
    //     frame_size: (3840, 2160),
    //     save_frames: true,
    //     ..Default::default()
    // });
}
