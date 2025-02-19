use std::f32;

use env_logger::Env;
use glam::{vec3, Vec3};
use ranim::animation::entity::creation::{uncreate, unwrite, write, Color};
use ranim::animation::entity::fading::{fade_in, fade_out};
use ranim::animation::entity::interpolate::interpolate;
use ranim::animation::timeline::Timeline;

use ranim::color::palettes::manim;
use ranim::items::svg_item::SvgItem;
use ranim::items::vitem::{Arc, Polygon};
use ranim::items::Rabject;
use ranim::{prelude::*, typst_svg, AppOptions, SceneDesc, TimelineConstructor};

const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

struct MainScene;

impl TimelineConstructor for MainScene {
    fn desc() -> ranim::SceneDesc {
        SceneDesc {
            name: "basic".to_string(),
        }
    }
    fn construct(&mut self, timeline: &mut Timeline) {
        let mut text = Rabject::new(SvgItem::from_svg(&typst_svg!(
            r#"
        #align(center)[
            #text(60pt)[Ranim]

            #text(20pt)[Hello 你好]
        ]
        "#
        )));
        text.set_fill_opacity(0.8).shift(Vec3::NEG_Y * 200.0);

        let mut svg = Rabject::new(SvgItem::from_svg(SVG));
        svg.scale(Vec3::splat(2.0)).shift(vec3(0.0, 200.0, 0.0));

        timeline.forward(0.2);
        timeline.play(fade_in(&svg));

        // [ranim_text] write -> 0.5s wait -> unwrite
        let mut text_scaled = timeline.play(write(&text).with_duration(3.0));
        text_scaled.scale(Vec3::splat(2.0));
        let text = timeline.play(interpolate(&text, &text_scaled));
        timeline.forward(0.5);
        timeline.play(unwrite(&text).with_duration(3.0));

        timeline.play(fade_out(&svg));

        let mut polygon = Polygon(vec![
            vec3(0.0, 0.0, 0.0),
            vec3(-100.0, -300.0, 0.0),
            vec3(0.0, 700.0, 0.0),
            vec3(200.0, 300.0, 0.0),
            vec3(500.0, 0.0, 0.0),
        ])
        .build();
        polygon
            .set_color(color!("#FF8080FF"))
            .set_fill_opacity(0.5)
            .rotate(std::f32::consts::FRAC_PI_2, Vec3::Z);

        // [polygon] 0.5s wait -> fade in -> 0.5s wait
        timeline.forward(0.5);
        timeline.play(fade_in(&polygon));
        timeline.forward(0.5);

        let mut arc = Arc {
            angle: f32::consts::PI / 3.0,
            radius: 100.0,
        }
        .build();
        arc.set_stroke_color(manim::BLUE_C);
        // [polygon] interpolate [svg] -> 0.5s wait
        let arc = timeline.play(interpolate(&polygon, &arc));
        timeline.forward(0.5);

        // [svg] fade_out -> 0.5s wait
        timeline.play(uncreate(&arc));
        timeline.forward(0.5);
    }
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=info,ranim=trace"))
        .init();

    MainScene.render(&AppOptions::default());
}
