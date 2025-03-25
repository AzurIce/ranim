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
        //     #text(font: "LXGW Bright")[有意思]

        //     #text(font: "LXGW Bright")[真的是人用的]

        //     #text(font: "LXGW Bright")[『我』的『软件』]
        // ]"#
        // ));
        let mut nvitem = Group::<NVItem>::from_svg(typst_svg!(r#"#text(font: "LXGW Bright")[软]"#));
        // let mut nvitem = NVItemBuilder::new();
        // nvitem.move_to(vec3(-3.4890716, 2.2969427, 0.0));
        // nvitem.cubic_to(
        //     vec3(-3.5152762, 2.2969427, 0.0),
        //     vec3(-3.5327399, 2.2794755, 0.0),
        //     vec3(-3.5327399, 2.2445414, 0.0),
        // );
        // // nvitem.close_path();
        // let mut nvitem = nvitem.build();
        nvitem
            .scale_to(ScaleHint::PorportionalHeight(8.0))
            .put_center_on(Vec3::ZERO);

        // nvitem
        //     .scale_to(ScaleHint::PorportionalWidth(3.8))
        //     .put_center_on(Vec3::X * 2.0);
        // let nvitem = nvitem[0].get_partial(0.0..0.15);
        // dbg!(nvitem.nvpoints.len());
        // println!("{:?}", nvitem.nvpoints);
        // nvitem
        //     .scale_to(ScaleHint::PorportionalWidth(3.8))
        //     .put_center_on(Vec3::X * 2.0);

        // let _vitem = timeline.insert_group(vitem);
        let _nvitem = timeline.insert_group(nvitem);
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
        // let _nvitem = timeline.insert(nvitem);
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

#[cfg(test)]
mod test {
    use glam::{DVec2, Vec2, dvec2, dvec3, vec2, vec3};

    const P: [Vec2; 4] = [
        vec2(450.0053, 540.0),
        vec2(-90.075745, 540.0),
        vec2(-450.00528, 179.99673),
        vec2(-450.00528, -539.99994),
    ];
    const P_DOUBLE: [DVec2; 4] = [
        dvec2(450.0053, 540.0),
        dvec2(-90.075745, 540.0),
        dvec2(-450.00528, 179.99673),
        dvec2(-450.00528, -539.99994),
    ];

    #[derive(Debug)]
    struct SolveCubicRes {
        n: u32,
        root: [f32; 3],
    }

    #[derive(Debug)]
    struct SolveCubicResDouble {
        n: u32,
        root: [f64; 3],
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
        dbg!((-(-27.0 / p3).sqrt() * q / 2.0).clamp(-1.0, 1.0).acos());
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

    fn solve_cubic_double(a: f64, b: f64, c: f64) -> SolveCubicResDouble {
        dbg!(a, b, c);
        let p = b - a * a / 3.0;
        let p3 = p * p * p;
        dbg!(p);
        dbg!(p3);

        let q = a * (2.0 * a * a - 9.0 * b) / 27.0 + c;
        let _d = q * q + 4.0 * p3 / 27.0;
        let offset = -a / 3.0;

        let u = (-p / 3.0).sqrt();
        let v = (-(-27.0 / p3).sqrt() * q / 2.0).acos() / 3.0;
        dbg!((-27.0 / p3).sqrt());
        dbg!(-(-27.0 / p3).sqrt() * q / 2.0);
        dbg!((-(-27.0 / p3).sqrt() * q / 2.0).acos());
        dbg!(v);
        let m = v.cos();
        let n = v.sin() * 1.732050808;

        let r = dvec3(m + m, -n - m, n - m) * u + offset;

        // let f = ((r + a) * r + b) * r + c;
        // let f_prime = (3.0 * r + 2.0 * a) * r + b;

        // r -= f / f_prime;

        SolveCubicResDouble {
            n: 3,
            root: [r.x, r.y, r.z],
        }
    }

    #[test]
    fn test_f() {
        let x: f32 = 0.009869999999978063;
        let x_double: f64 = 0.009869999999978063;
        dbg!(x, x_double);
    }

    #[test]
    fn test_precision() {
        let pos = vec2(0.0, 0.0);
        let cu = (P[3].y + P[1].y * 3.0) - (P[2].y * 3.0 + P[0].y);
        // let cu = 0.00987;
        let qu = 3.0 * P[0].y - 6.0 * P[1].y + 3.0 * P[2].y;
        let li = -3.0 * P[0].y + 3.0 * P[1].y;
        let co = P[0].y - pos.y;
        dbg!(cu, qu, li, co);

        let res = solve_cubic(qu / cu, li / cu, co / cu);
        println!("{:?}", res);
        println!("##########");

        let pos = dvec2(0.0, 0.0);
        let cu = -P_DOUBLE[0].y + 3.0 * P_DOUBLE[1].y - 3.0 * P_DOUBLE[2].y + P_DOUBLE[3].y;
        let qu = 3.0 * P_DOUBLE[0].y - 6.0 * P_DOUBLE[1].y + 3.0 * P_DOUBLE[2].y;
        let li = -3.0 * P_DOUBLE[0].y + 3.0 * P_DOUBLE[1].y;
        let co = P_DOUBLE[0].y - pos.y;
        dbg!(cu, qu, li, co);

        let res_double = solve_cubic_double(qu / cu, li / cu, co / cu);
        println!("{:?}", res_double);
    }

    
    #[test]
    fn foo() {
        // 仅使用f32计算，并应用浮点误差减少技术
        let pos = vec2(0.0, 0.0);

        // 1. 更精确的计算顺序
        // 将大小相近的值分组，避免大数吞噬小数
        let cu = (P[3].y - P[2].y * 3.0) + (P[1].y * 3.0 - P[0].y);
        
        // 2. 使用Horner方法重写多项式系数计算
        let qu = 3.0 * (P[0].y + P[2].y - 2.0 * P[1].y);
        let li = 3.0 * (P[1].y - P[0].y);
        let co = P[0].y - pos.y;
        
        // 3. 归一化系数，提高数值稳定性
        let max_coeff = cu.abs().max(qu.abs()).max(li.abs()).max(co.abs());
        let (norm_cu, norm_qu, norm_li, norm_co) = if max_coeff > 0.0 && false {
            (cu / max_coeff, qu / max_coeff, li / max_coeff, co / max_coeff)
        } else {
            (cu, qu, li, co)
        };
        
        // 6. 解方程
        let a = norm_qu / norm_cu;
        let b = norm_li / norm_cu;
        let c = norm_co / norm_cu;
        let res = solve_cubic(a, b, c);
        
        // 7. 牛顿迭代优化根的精度
        let mut improved_roots = [0.0; 3];
        for i in 0..3 {
            let mut x = res.root[i];
            // 应用2-3次牛顿迭代来提高根的精度
            for _ in 0..6 {
                // f(x) = x³ + ax² + bx + c
                let f = ((x + a) * x + b) * x + c;
                // f'(x) = 3x² + 2ax + b
                let f_prime = (3.0 * x + 2.0 * a) * x + b;
                
                if f_prime.abs() < 1e-10 {
                    break; // 避免除以接近零的数
                }
                
                let delta = f / f_prime;
                x -= delta;
                
                if delta.abs() < 1e-8 {
                    break; // 收敛，可以提前停止
                }
            }
            improved_roots[i] = x;
        }
        
        // 输出和对比结果
        println!("原始解算结果: {:?}", res);
        println!("优化后的根: {:?}", improved_roots);
        println!("直接计算的cu: {}", cu);
        
        // 检验根的准确性
        for (i, &root) in improved_roots.iter().enumerate() {
            let original = ((root + a) * root + b) * root + c;
            println!("根 {} 的方程验证值: {}", i, original);
        }
    }
}
