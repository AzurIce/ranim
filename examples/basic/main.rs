use std::f32;

use env_logger::Env;
use glam::{vec3, Vec3};
use ranim::animation::creation::{Color, CreationAnim, WritingAnim};
use ranim::animation::fading::FadingAnim;
use ranim::animation::transform::TransformAnim;
use ranim::color::palettes::manim;
use ranim::items::svg_item::SvgItem;
use ranim::items::vitem::{Arc, Polygon};
use ranim::timeline::Timeline;
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
        timeline.forward(0.2);

        let mut svg = SvgItem::from_svg(SVG);
        svg.scale(Vec3::splat(2.0)).shift(vec3(0.0, 200.0, 0.0));
        let mut svg = timeline.insert(svg);
        timeline.play(svg.fade_in());

        let mut text = SvgItem::from_svg(&typst_svg!(
            r#"
        #align(center)[
            #text(60pt)[Ranim]

            #text(20pt)[Hello 你好]
        ]
        "#
        ));
        text.set_fill_opacity(0.8).shift(Vec3::NEG_Y * 200.0);
        let mut text = timeline.insert(text);

        timeline.play(text.write().with_duration(3.0));

        timeline.play(
            text.transform(|data| {
                data.scale(Vec3::splat(2.0));
            })
            .apply(), // `apply` will apply the animation's effect to rabject's data
        );

        timeline.forward(0.5);
        timeline.play(text.unwrite().with_duration(3.0));
        timeline.play(svg.fade_out());

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
        let mut polygon = timeline.insert(polygon);
        timeline.play(polygon.fade_in());
        timeline.forward(0.5);

        let mut arc = Arc {
            angle: f32::consts::PI / 3.0,
            radius: 100.0,
        }
        .build();
        arc.set_stroke_color(manim::BLUE_C);
        let mut arc = timeline.insert(arc);
        // [polygon] interpolate [svg] -> 0.5s wait

        let polygon_data = polygon.data.clone();
        drop(polygon);
        timeline.play(arc.transform_from(polygon_data));
        timeline.forward(0.5);

        // [svg] fade_out -> 0.5s wait
        timeline.play(arc.uncreate());
        timeline.forward(0.5);
    }
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=info,ranim=info")).init();

    MainScene.render(&AppOptions::default());
}
