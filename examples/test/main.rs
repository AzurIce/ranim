#![allow(clippy::all)]
#![allow(unused_imports)]
use std::{f64::consts::PI, time::Duration};

use ::color::palette::css;
use glam::{DVec3, dvec3};
use log::LevelFilter;
use ranim::{
    animation::{
        creation::{CreationAnim, WritingAnim},
        fading::FadingAnimSchedule,
        transform::{TransformAnim, TransformAnimSchedule},
    },
    color::palettes::manim::{self, RED_C},
    components::{Anchor, ScaleHint},
    items::{
        camera_frame::CameraFrame,
        group::Group,
        vitem::{arrow::Arrow, Circle, Polygon, Square, VItem},
    },
    prelude::*,
    typst_svg,
};

// const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[scene]
struct TestScene;

impl TimelineConstructor for TestScene {
    fn construct(self, timeline: &RanimTimeline, _camera: &mut Rabject<CameraFrame>) {
        let arrow = Arrow::new(-3.0 * DVec3::X, 3.0 * DVec3::Y);
        let mut arrow = timeline.insert(arrow);

        timeline.play(arrow.transform(|data| {
            data.set_color(RED_C);
            data.put_start_and_end_on(DVec3::NEG_Y, DVec3::Y);
        }));
        timeline.sync();
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
    // render_scene_at_sec(TestScene, 0.0, "test.png", &AppOptions::default());
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
