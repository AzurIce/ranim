#![allow(clippy::all)]
#![allow(unused_imports)]
use std::{f64::consts::PI, time::Duration};

use ::color::palette::css;
use env_logger::Env;
use glam::{DVec3, dvec3};
use ranim::{
    animation::{
        creation::{CreationAnim, WritingAnim},
        transform::{TransformAnim, TransformAnimSchedule},
    },
    components::{Anchor, ScaleHint},
    items::{
        Arrow,
        camera_frame::CameraFrame,
        group::Group,
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
        let arrow = Arrow::new();
        let mut arrow = timeline.insert(arrow);
        timeline.play(arrow.iter_mut().lagged_anim(0.2, |part| part.fade_in()));
        timeline.forward(1.0);
        // let _item = Square(500.0).build();
        // let mut vitem = Group::<VItem>::from_svg(typst_svg!(
        //     r#"#align(center)[
        //     #text(font: "LXGW Bright")[有意思]

        //     #text(font: "LXGW Bright")[真的是人用的]

        //     #text(font: "LXGW Bright")[『我』的『软件』]
        // ]"#
        // ));
        // vitem
        //     .scale_to(ScaleHint::PorportionalHeight(8.0))
        //     .put_center_on(DVec3::ZERO);
        // let vitem = vitem[0].clone().get_partial(0.0..0.4);
        // println!("vpoints: {:?}", vitem.vpoints);
        // println!("close_path: {:?}", vitem.vpoints.get_closepath_flags());
        // let _vitem = timeline.insert(vitem);

        // let mut border = Square(1.0).build();
        // border
        //     .scale_to(ScaleHint::Size(8.0, 8.0))
        //     .put_center_on(DVec3::ZERO);
        // let _border = timeline.insert(border);

        // // let _vitem = timeline.insert(vitem);
        // timeline.forward(1.0);
        // timeline.sync();
        // timeline.play(camera.transform(|camera| camera.fovy = PI / 4.0));

        timeline.sync();
    }
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("test=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("test=info,ranim=trace")).init();
    // println!("main");
    // render_scene(
    //     TestScene,
    //     &AppOptions {
    //         frame_rate: 60,
    //         ..AppOptions::default()
    //     },
    // );
    #[cfg(not(feature = "app"))]
    render_scene_at_sec(TestScene, 0.0, "test.png", &AppOptions::default());

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
