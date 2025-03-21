#![allow(clippy::all)]
#![allow(unused_imports)]
use std::{f32::consts::PI, time::Duration};

use env_logger::Env;
use glam::{Vec3, vec3};
use ranim::{
    animation::{
        creation::{CreationAnim, WritingAnim},
        transform::{TransformAnim, TransformAnimSchedule},
    },
    components::{vpoint::VPointSliceMethods, Anchor, ScaleHint},
    items::{
        camera_frame::CameraFrame, group::Group, nvitem::NVItem, svg_item::SvgItem, vitem::{Square, VItem}
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
        // let mut text = Group::<VItem>::from_svg(typst_svg!(r#"#text(font: "LXGW Bright")[意软]"#));
        // let mut text = Group::<VItem>::from_svg(typst_svg!(r#"#text(font: "LXGW Bright")[有意思]"#));
        // let mut text = Group::<VItem>::from_svg(typst_svg!(r#"#align(center)[
        //     #text(font: "LXGW Bright")[有意思]

        //     #text(font: "LXGW Bright")[真的是人用的]

        //     #text(font: "LXGW Bright")[『我』的『软件』]
        // ]"#));
        // println!("{}", text.len());
        // let vpoints = text[0].vpoints.get(0..94).unwrap();
        // println!("{:?}", vpoints);
        // println!("{:?}", vpoints.get_closepath_flags());
        // text.scale_to(ScaleHint::PorportionalHeight(1.5));
        // let text = text[14].clone();
        // let vpoints = text.vpoints.get(0..).unwrap();
        // println!("{:?}", vpoints);
        // println!("{:?}", vpoints.get_closepath_flags());
        let item = NVItem::from_nvpoints(vec![
            [
                vec3(0.0, 0.0, 0.0),
                vec3(0.0, 0.0, 0.0),
                vec3(0.0, 2.0, 0.0),
            ],
            [
                vec3(2.0, 0.0, 0.0),
                vec3(2.0, 2.0, 0.0),
                vec3(2.0, 4.0, 0.0),
            ],
            // [
            //     vec3(-2.0, 4.0, 0.0),
            //     vec3(-2.0, 2.0, 0.0),
            //     vec3(-2.0, 2.0, 0.0),
            // ],
            [
                vec3(-2.0, 4.0, 0.0),
                vec3(-2.0, 2.0, 0.0),
                vec3(-2.0, 0.0, 0.0),
            ],
            [
                vec3(-2.0, 0.0, 0.0),
                vec3(0.0, 0.0, 0.0),
                vec3(0.0, 0.0, 0.0),
            ],
        ]);
        // let item = Square(4.0).build();

        let _item = timeline.insert(item);

        // let mut _text = timeline.insert_group(text);
        // let mut _text = timeline.insert(text);

        timeline.forward(1.0);
        timeline.sync();
    }
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("test=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("test=info,ranim=trace")).init();
    // println!("main");
    render_scene(
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
