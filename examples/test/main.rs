#![allow(clippy::all)]
#![allow(unused)]
use std::{f64::consts::PI, time::Duration};

use ::color::palette::css;
use glam::{DVec3, dvec3};
use log::LevelFilter;
use ranim::{
    animation::{
        creation::{CreationAnim, WritingAnim, WritingAnimSchedule},
        fading::{FadingAnim, FadingAnimSchedule},
        transform::{TransformAnim, TransformAnimSchedule},
    },
    color::palettes::manim::{self, BLUE_C, RED_C},
    components::{Anchor, ScaleHint},
    items::{
        camera_frame::CameraFrame,
        group::Group,
        vitem::{
            self, VItem,
            geometry::{Polygon, Rectangle, Square},
        },
    },
    prelude::*,
    typst_svg,
};

// const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[scene]
struct TestScene;

impl TimelineConstructor for TestScene {
    fn construct(self, timeline: &RanimTimeline, _camera: &mut Rabject<CameraFrame>) {
        // let mut text = Group::<VItem>::from_svg(typst_svg!(
        //     r#"#align(center)[
        //     #text(10pt, font: "LXGW Bright")[R]
        // ]"#
        // ));

        let square_a = Square::new(1.0);
        let rectangle_a = Rectangle::new(1.0, 2.0);
        {
            let mut square = timeline.insert(VItem::from(square_a.clone()));
            let mut rectangle = timeline.insert(VItem::from(rectangle_a.clone()));
            timeline.play_and_hide(square.write());
            timeline.play_and_hide(rectangle.write());
            timeline.sync();
        }

        let mut rectangle_b = timeline.insert(Rectangle::from(square_a));
        let square_b = Square::new(2.0);
        {
            let mut square = timeline.insert(Rectangle::from(square_b.clone()));
            timeline.play_and_hide(square.transform_from(rectangle_a.clone()));
        }
        let mut square_b = timeline.insert(square_b);

        timeline.forward(1.0);

        timeline.play(
            rectangle_b
                .transform(|rectangle| {
                    rectangle.scale_to(ScaleHint::X(2.0));
                })
                .apply(),
        );
        timeline.sync();

        timeline.play(square_b.fade_out());
        timeline.play(rectangle_b.fade_out());

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
