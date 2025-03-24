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
    render_scene_at_sec(TestScene, 0.0, "test.png", &AppOptions {
        frame_rate: 60,
        ..AppOptions::default()
    });
    // TestScene.render(&AppOptions {
    //     frame_rate: 60,
    //     frame_size: (3840, 2160),
    //     save_frames: true,
    //     ..Default::default()
    // });
}

#[cfg(test)]
mod test {
    use glam::{DVec2, Vec2, vec2, dvec2, vec3, dvec3};

    const P: [Vec2; 4] = [
        vec2(450.0053, 540.0),
        vec2(-90.075745, 540.0),
        vec2(-450.00528, 179.99673),
        vec2(-450.00528, -539.99994),
    ];


    #[derive(Debug)]
    struct SolveCubicRes {
        n: u32,
        root: [f32; 3],
    }

    fn solve_cubic(a: f32, b: f32, c: f32) -> SolveCubicRes {
        dbg!(a, b, c);
        let p = b - a * a / 3.0;
        let p3 = p * p * p;
        dbg!(p);
        dbg!(p3);

        let q = a * (2.0 * a * a - 9.0 * b) / 27.0 + c;
        let _d = q * q + 4.0 * p3 / 27.0;
        let offset = -a / 3.0;

        let u = (-p / 3.0).sqrt();
        let v = (-(-27.0 / p3).sqrt() * q / 2.0).clamp(-1.0, 1.0).acos() / 3.0;
        dbg!((-27.0 / p3).sqrt());
        dbg!(-(-27.0 / p3).sqrt() * q / 2.0);
        dbg!((-(-27.0 / p3).sqrt() * q / 2.0).acos());
        dbg!(v);
        let m = v.cos();
        let n = v.sin() * 1.732050808;

        let r = vec3(m + m, -n - m, n - m) * u + offset;

        // let f = ((r + a) * r + b) * r + c;
        // let f_prime = (3.0 * r + 2.0 * a) * r + b;

        // r -= f / f_prime;

        SolveCubicRes {
            n: 3,
            root: [r.x, r.y, r.z],
        }
    }

    #[test]
    fn foo() {
        let pos = vec2(0.0, 0.0);
        let cu = -P[0].y + 3.0 * P[1].y - 3.0 * P[2].y + P[3].y;
        let qu = 3.0 * P[0].y - 6.0 * P[1].y + 3.0 * P[2].y;
        let li = -3.0 * P[0].y + 3.0 * P[1].y;
        let co = P[0].y - pos.y;
        dbg!(cu, qu, li, co);

        let res = solve_cubic(qu / cu, li / cu, co / cu);
        println!("{:?}", res);
    }
}
