use std::f64::consts::PI;

use glam::{DVec3, dvec2, dvec3};
use itertools::Itertools;
use ranim::{
    animation::{creation::WritingAnim, transform::TransformAnim, LaggedAnim},
    color::palettes::manim,
    components::{Anchor, ScaleHint},
    items::{
        vitem::{
            geometry::{Polygon, Rectangle, Square}, svg::SvgItem, VItem
        }, Group
    },
    prelude::*,
    timeline::{TimeMark, TimelineTrait, TimelinesFunc},
    typst_svg,
    utils::rate_functions::{linear, smooth},
};

fn build_logo(logo_width: f64) -> [VItem; 6] {
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
struct RanimLogoScene;

impl TimelineConstructor for RanimLogoScene {
    fn construct(self, r: &mut RanimScene, _r_cam: TimelineId<CameraFrame>) {
        let frame_size = dvec2(8.0 * 16.0 / 9.0, 8.0);
        let logo_width = frame_size.y * 0.618;

        let logo = build_logo(logo_width);
        let logo = logo.map(|item| r.init_timeline(item));

        let ranim_text = Group::<VItem>::from(
            SvgItem::new(typst_svg!(
                r#"
#align(center)[
    #text(10pt, font: "LXGW Bright")[Ranim]
]"#
            ))
            .with(|text| {
                text.set_color(manim::WHITE)
                    .scale_to(ScaleHint::PorportionalY(1.0))
                    .put_center_on(DVec3::NEG_Y * 2.5);
            }),
        );
        let r_ranim_text = r.init_timeline(ranim_text);

        logo.iter().for_each(|item| {
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
        logo.iter()
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
        r.timeline_mut(&r_ranim_text).forward(0.5);
        r.timeline_mut(&r_ranim_text).play_with(|text| {
            text.lagged(0.2, |item| {
                item.write().with_duration(2.0).with_rate_func(linear)
            })
            .with_duration(2.0)
        });
        r.timelines_mut().sync();

        r.insert_time_mark(r.cur_sec(), TimeMark::Capture("preview.png".to_string()));
        r.timelines_mut().forward(1.0);

        logo.iter().for_each(|r_logo| {
            r.timeline_mut(r_logo)
                .play_with(|item| item.unwrite().with_duration(3.0).with_rate_func(smooth));
        });
        r.timeline_mut(&r_ranim_text).play_with(|text| {
            text.lagged(0.0, |item| {
                item.unwrite().with_duration(3.0).with_rate_func(linear)
            })
        });
    }
}

fn main() {
    #[cfg(feature = "app")]
    run_scene_app(RanimLogoScene);
    #[cfg(not(feature = "app"))]
    render_scene(RanimLogoScene, &AppOptions::default());
}
