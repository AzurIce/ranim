use std::f64::consts::TAU;

use itertools::Itertools;
use ranim::{
    color::palettes::manim,
    glam::{DVec3, usizevec3},
    prelude::*,
};
use ranim_anims::transform::TransformAnim;
use ranim_core::animation::StaticAnim;
use ranim_items::vitem::{VItem, svg::SvgItem, typst::typst_svg};

#[scene(clear_color = "#000000")]
#[output(dir = "animating_pi")]
fn animating_pi(r: &mut RanimScene) {
    let cam = CameraFrame::default();
    let _r_cam = r.insert(cam.clone());

    let (w, h) = (10, 10);
    let mut pis = (0..h)
        .cartesian_product(0..w)
        .map(|(_, _)| {
            SvgItem::new(typst_svg("$pi$"))
                .with(|x| {
                    x.set_color(manim::WHITE);
                })
                .with(|x| {
                    x.scale_to_min(&[
                        ScaleHint::PorportionalY(cam.frame_height / h as f64),
                        ScaleHint::PorportionalX(cam.frame_height / w as f64),
                    ])
                    .discard()
                })
        })
        .collect::<Vec<_>>();
    pis.arrange_in_grid(
        usizevec3(10, 10, usize::MAX),
        DVec3::splat(8.0 / 10.0),
        DVec3::splat(0.2),
    )
    .move_to(DVec3::ZERO);

    let mut vitems = pis
        .into_iter()
        .flat_map(Vec::<VItem>::from)
        .collect::<Vec<_>>();
    vitems
        .scale_to(ScaleHint::PorportionalY(TAU - 0.25))
        .move_to(DVec3::ZERO);

    r.insert_with(|t| {
        t.play(vitems.clone().show())
            .forward(1.0)
            .play(
                vitems
                    .transform(|x| {
                        x.apply_complex_map(|c| c.exp());
                    })
                    .with_duration(5.0),
            )
            .forward(1.0)
            .play(
                vitems
                    .transform(|x| {
                        x.apply_point_func(|p| {
                            p.x += 0.5 * p.y.sin();
                            p.y += 0.5 * p.x.cos();
                        });
                    })
                    .with_duration(5.0),
            )
            .forward(1.0);
    });
    r.insert_time_mark(5.0, TimeMark::Capture("preview.png".to_string()));
}

#[test]
fn foo() {
    let svg = typst_svg("R");
    println!("{svg}");
}

#[test]
fn foo_2() {
    let svg = compile_typst_code("R");
    println!("{svg}");
}
