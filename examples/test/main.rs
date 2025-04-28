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
    color::palettes::manim,
    components::{Anchor, ScaleHint},
    items::{
        camera_frame::CameraFrame,
        group::Group,
        vitem::{Circle, Polygon, Square, VItem, arrow::Arrow},
    },
    prelude::*,
    typst_svg,
};

// const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[scene]
struct TestScene;

impl TimelineConstructor for TestScene {
    fn construct(self, timeline: &RanimTimeline, _camera: &mut Rabject<CameraFrame>) {
        let mut pentagon = Polygon(
            (0..=5)
                .map(|i| {
                    let angle = i as f64 / 5.0 * 2.0 * PI;
                    dvec3(angle.cos(), angle.sin(), 0.0) * 2.0
                })
                .collect(),
        )
        .build();
        pentagon
            .set_color(manim::RED_C)
            // .rotate(PI / 2.0, DVec3::Z)
            .set_stroke_width(2.0);
        let mut pentagon = timeline.insert(pentagon);

        let mut circle = Circle(2.0).build();
        circle.set_color(manim::BLUE_C).set_stroke_width(2.0);

        println!("{:?}", pentagon.data.vpoints);
        pentagon.data.align_with(&mut circle);
        println!("{:?}", pentagon.data.vpoints);

        timeline.play(pentagon.transform_to(circle));
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
