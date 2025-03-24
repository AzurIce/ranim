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
    components::{Anchor, ScaleHint, vpoint::VPointSliceMethods},
    items::{
        camera_frame::CameraFrame,
        group::Group,
        nvitem::{NVItem, NVItemBuilder},
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
        // let mut text = Group::<VItem>::from_svg(typst_svg!(r#"#text(font: "LXGW Bright")[意软]"#));
        // let mut text = Group::<VItem>::from_svg(typst_svg!(r#"#text(font: "LXGW Bright")[有意思]"#));
        // let mut vitem = Group::<VItem>::from_svg(typst_svg!(
        //     r#"#align(center)[
        //     #text(font: "LXGW Bright")[有意思]

        //     #text(font: "LXGW Bright")[真的是人用的]

        //     #text(font: "LXGW Bright")[『我』的『软件』]
        // ]"#
        // ));
        // vitem
        //     .scale_to(ScaleHint::PorportionalWidth(3.8))
        //     .put_center_on(Vec3::NEG_X * 2.0);
        // let mut nvitem = Group::<NVItem>::from_svg(typst_svg!(
        //     r#"#align(center)[
        //     #text(font: "LXGW Bright")[软]
        // ]"#
        // ));
        let mut nvitem = NVItemBuilder::new();
        nvitem.move_to(vec3(-3.4890716, 2.2969427, 0.0));
        nvitem.cubic_to(
            vec3(-3.5152762, 2.2969427, 0.0),
            vec3(-3.5327399, 2.2794755, 0.0),
            vec3(-3.5327399, 2.2445414, 0.0),
        );
        // nvitem.close_path();
        let mut nvitem = nvitem.build();

        nvitem
            .scale_to(ScaleHint::PorportionalHeight(8.0))
            .put_center_on(Vec3::ZERO);
        // let nvitem = nvitem[0].get_partial(0.0..0.15);
        // dbg!(nvitem.nvpoints.len());
        // println!("{:?}", nvitem.nvpoints);
        // nvitem
        //     .scale_to(ScaleHint::PorportionalWidth(3.8))
        //     .put_center_on(Vec3::X * 2.0);

        // let _vitem = timeline.insert_group(vitem);
        // let _nvitem = timeline.insert_group(nvitem);
        // println!("{}", text.len());
        // let vpoints = text[0].vpoints.get(0..94).unwrap();
        // println!("{:?}", vpoints);
        // println!("{:?}", vpoints.get_closepath_flags());
        // text.scale_to(ScaleHint::PorportionalHeight(1.5));
        // let text = text[14].clone();
        // let vpoints = text.vpoints.get(0..).unwrap();
        // println!("{:?}", vpoints);
        // println!("{:?}", vpoints.get_closepath_flags());
        // let vitem = VItem::from_vpoints(vec![
        //     vec3(0.0, 0.0, 0.0),
        //     vec3(0.0, 2.0, 0.0),
        //     vec3(2.0, 2.0, 0.0),
        // ]);
        // let nvitem = NVItem::from_nvpoints(vec![
        //     [
        //         vec3(0.0, 0.0, 0.0),
        //         vec3(0.0, 0.0, 0.0),
        //         vec3(0.0, 2.0, 0.0),
        //     ],
        //     [
        //         vec3(0.0, 2.0, 0.0),
        //         vec3(2.0, 2.0, 0.0),
        //         vec3(2.0, 2.0, 0.0),
        //     ],
        // ]);
        // let item = Square(4.0).build();

        // let _vitem = timeline.insert(vitem);
        let _nvitem = timeline.insert(nvitem);
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
    render_scene_at_sec(
        TestScene,
        0.0,
        "test.png",
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
