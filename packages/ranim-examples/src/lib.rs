//! This crate contains all ranim examples

use std::f64::consts::PI;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use itertools::Itertools;
use ranim::{
    animation::{
        creation::WritingAnim, fading::FadingAnim, lagged::LaggedAnim, transform::TransformAnim,
    },
    color::palettes::manim,
    components::{Anchor, ScaleHint},
    glam::{DVec3, dvec2, dvec3},
    items::{
        Group,
        vitem::{
            VItem,
            geometry::{Circle, Polygon, Rectangle, Square},
            svg::SvgItem,
            typst::typst_svg,
        },
    },
    prelude::*,
    timeline::TimeMark,
    utils::rate_functions::{linear, smooth},
};

pub mod tutorial {
    pub mod extract;
    pub mod getting_started;
}
pub mod vitem {
    pub mod geometry;
    pub mod svg;
}
pub mod algo {
    pub mod hanoi;
    pub mod sort;
}
pub mod camera;

// MARK: hello_ranim
#[scene]
#[preview]
#[wasm_demo_doc]
#[output(dir = "hello_ranim")]
/// Hello Ranim!
pub fn hello_ranim(r: &mut RanimScene) {
    let _r_cam = r.insert_and_show(CameraFrame::default());
    let square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });

    let r_square = r.insert(square);
    {
        let timeline = r.timeline_mut(&r_square);
        timeline.play_with(|square| square.fade_in());
    };

    let circle = Circle::new(2.0).with(|circle| {
        circle
            .set_color(manim::RED_C)
            .rotate(-PI / 4.0 + PI, DVec3::Z);
    });

    let r_vitem = r.map(r_square, VItem::from);
    {
        let timeline = r.timeline_mut(&r_vitem);
        timeline.play_with(|state| state.transform_to(circle.into()));
        timeline.forward(1.0);
        let circle = timeline.state().clone();
        timeline.play_with(|circle| circle.unwrite());
        timeline.play(circle.write());
        timeline.play_with(|circle| circle.fade_out());
    };

    r.insert_time_mark(3.7, TimeMark::Capture("preview.png".to_string()));
}

// MARK: ranim_logo
pub fn build_logo(logo_width: f64) -> [VItem; 6] {
    let red_bg_rect = Rectangle::new(logo_width / 2.0, logo_width).with(|rect| {
        rect.set_color(manim::RED_C.with_alpha(0.5))
            .put_center_on(dvec3(-logo_width / 4.0, 0.0, 0.0));
    });
    let red_rect = Rectangle::new(logo_width / 4.0, logo_width).with(|rect| {
        rect.set_color(manim::RED_C)
            .put_anchor_on(Anchor::edge(1, 0, 0), dvec3(-logo_width / 4.0, 0.0, 0.0));
    });

    let green_bg_sq = Square::new(logo_width / 2.0).with(|sq| {
        sq.set_color(manim::GREEN_C.with_alpha(0.5))
            .put_center_on(dvec3(logo_width / 4.0, logo_width / 4.0, 0.0));
    });
    let green_triangle = Polygon::new(vec![
        dvec3(0.0, logo_width / 2.0, 0.0),
        dvec3(logo_width / 2.0, logo_width / 2.0, 0.0),
        dvec3(logo_width / 2.0, 0.0, 0.0),
    ])
    .with(|tri| {
        tri.set_color(manim::GREEN_C);
    }); // ◥

    let blue_bg_sq = Square::new(logo_width / 2.0).with(|sq| {
        sq.set_color(manim::BLUE_C.with_alpha(0.5))
            .put_center_on(dvec3(logo_width / 4.0, -logo_width / 4.0, 0.0));
    });
    let blue_triangle = green_triangle.clone().with(|tri| {
        tri.set_color(manim::BLUE_C)
            .rotate(PI, DVec3::Z)
            .shift(DVec3::NEG_Y * logo_width / 2.0);
    }); // ◣

    [
        VItem::from(red_bg_rect),
        VItem::from(red_rect),
        VItem::from(green_bg_sq),
        VItem::from(green_triangle),
        VItem::from(blue_bg_sq),
        VItem::from(blue_triangle),
    ]
}

#[scene]
#[wasm_demo_doc]
#[preview]
#[output(dir = "ranim_logo")]
/// A scene shows the logo of ranim
pub fn ranim_logo(r: &mut RanimScene) {
    let _r_cam = r.insert_and_show(CameraFrame::default());
    let frame_size = dvec2(8.0 * 16.0 / 9.0, 8.0);
    let logo_width = frame_size.y * 0.618;

    let logo = build_logo(logo_width);
    let r_logo = logo.map(|item| r.insert(item));

    let ranim_text = Group::<VItem>::from(
        SvgItem::new(typst_svg(
            r#"
#align(center)[
    #text(10pt, font: "LXGW Bright")[Ranim]
]"#,
        ))
        .with(|text| {
            text.set_color(manim::WHITE)
                .scale_to(ScaleHint::PorportionalY(1.0))
                .put_center_on(DVec3::NEG_Y * 2.5);
        }),
    );
    let r_ranim_text = r.insert(ranim_text);

    r_logo.iter().for_each(|item| {
        r.timeline_mut(item)
            .play_with(|item| item.write().with_duration(3.0).with_rate_func(smooth));
    });
    r.timelines_mut().sync();

    let gap_ratio = 1.0 / 60.0;
    let gap = logo_width * gap_ratio;
    let scale = (logo_width - gap * 2.0) / logo_width;
    let scale = [
        dvec3(scale, 1.0, 1.0),
        dvec3(scale, scale, 1.0),
        dvec3(scale, scale, 1.0),
    ];
    let anchor = [
        Anchor::edge(-1, 0, 0),
        Anchor::edge(1, 1, 0),
        Anchor::edge(1, -1, 0),
    ];
    r_logo
        .iter()
        .chunks(2)
        .into_iter()
        .zip(scale.into_iter().zip(anchor))
        .for_each(|(chunk, (scale, anchor))| {
            let chunk = chunk.collect_array::<2>().unwrap();
            r.timeline_mut(&chunk).iter_mut().for_each(|timeline| {
                timeline.play_with(|item| {
                    item.transform(|data| {
                        data.scale_by_anchor(scale, anchor)
                            .scale_by_anchor(dvec3(0.9, 0.9, 1.0), Anchor::ORIGIN)
                            .shift(dvec3(0.0, 1.3, 0.0));
                    })
                    .with_rate_func(smooth)
                });
            });
        });
    r.timeline_mut(&r_ranim_text)
        .forward(0.5)
        .play_with(|text| {
            text.lagged(0.2, |item| {
                item.write().with_duration(2.0).with_rate_func(linear)
            })
            .with_duration(2.0)
        });
    r.timelines_mut().sync();

    r.insert_time_mark(
        r.timelines().max_total_secs(),
        TimeMark::Capture("preview.png".to_string()),
    );
    r.timelines_mut().forward(1.0);

    r_logo.iter().for_each(|r_logo_part| {
        r.timeline_mut(r_logo_part)
            .play_with(|item| item.unwrite().with_duration(3.0).with_rate_func(smooth));
    });
    r.timeline_mut(&r_ranim_text).play_with(|text| {
        text.lagged(0.0, |item| {
            item.unwrite().with_duration(3.0).with_rate_func(linear)
        })
    });
}

// MARK: palettes
#[scene]
#[wasm_demo_doc]
#[preview]
#[output(dir = "palettes")]
/// An example shows manim's palettes
pub fn palettes(r: &mut RanimScene) {
    use ranim::color::palettes::manim::*;
    let _r_cam = r.insert_and_show(CameraFrame::default());
    let frame_size = dvec2(8.0 * 16.0 / 9.0, 8.0);
    let padded_frame_size = frame_size * 0.9;

    let colors = vec![
        vec![BLUE_E, BLUE_D, BLUE_C, BLUE_B, BLUE_A],
        vec![TEAL_E, TEAL_D, TEAL_C, TEAL_B, TEAL_A],
        vec![GREEN_E, GREEN_D, GREEN_C, GREEN_B, GREEN_A],
        vec![YELLOW_E, YELLOW_D, YELLOW_C, YELLOW_B, YELLOW_A],
        vec![GOLD_E, GOLD_D, GOLD_C, GOLD_B, GOLD_A],
        vec![RED_E, RED_D, RED_C, RED_B, RED_A],
        vec![MAROON_E, MAROON_D, MAROON_C, MAROON_B, MAROON_A],
        vec![PURPLE_E, PURPLE_D, PURPLE_C, PURPLE_B, PURPLE_A],
        vec![GREY_E, GREY_D, GREY_C, GREY_B, GREY_A],
        vec![WHITE, BLACK, GREEN_SCREEN],
        vec![GREY_BROWN, LIGHT_BROWN, PINK, LIGHT_PINK, ORANGE],
    ];

    let padded_frame_start = dvec2(padded_frame_size.x / -2.0, padded_frame_size.y / -2.0);
    let h_step = padded_frame_size.y / colors.len() as f64;

    let squares = colors
        .iter()
        .enumerate()
        .flat_map(|(i, row)| {
            let y = i as f64 * h_step;
            let w_step = padded_frame_size.x / row.len() as f64;
            row.iter().enumerate().map(move |(j, color)| {
                let x = j as f64 * w_step;
                Rectangle::new(w_step as f64, h_step as f64).with(|rect| {
                    rect.stroke_width = 0.0;

                    rect.set_color(*color).put_anchor_on(
                        Anchor::edge(-1, -1, 0),
                        padded_frame_start.extend(0.0) + dvec3(x, y, 0.0),
                    );
                })
            })
        })
        .collect::<Group<_>>();
    r.insert_and_show(squares);
    r.insert_time_mark(0.0, TimeMark::Capture("preview.png".to_string()));
    r.timelines_mut().forward(0.01);
}
