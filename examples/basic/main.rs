use std::time::Instant;

use bevy_color::Srgba;
use env_logger::Env;
use glam::{vec3, Vec3};
use log::info;
use ranim::animation::entity::creation::{uncreate, Color};
use ranim::animation::entity::fading::{fade_in, fade_out};
use ranim::animation::entity::interpolate::interpolate;
use ranim::animation::Timeline;

use ranim::items::svg_item::SvgItem;
use ranim::items::vitem::{Arc, Polygon};
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
        let t = Instant::now();
        info!("running...");

        let mut ranim_text = SvgItem::from_svg(&typst_svg!("#text(60pt)[Ranim]"));
        ranim_text.shift(-ranim_text.get_bounding_box()[1]);

        // // 0.5s wait -> fade in -> 0.5s wait
        // app.wait(Duration::from_secs_f32(0.5));
        // let ranim_text = app.play(ranim_text, creation::write());
        // app.wait(Duration::from_secs_f32(0.5));

        let mut polygon = Polygon(vec![
            vec3(0.0, 0.0, 0.0),
            vec3(-100.0, -300.0, 0.0),
            vec3(0.0, 700.0, 0.0),
            vec3(200.0, 300.0, 0.0),
            vec3(500.0, 0.0, 0.0),
        ])
        .build();
        polygon
            .set_color(Srgba::hex("FF8080FF").unwrap())
            .set_fill_opacity(0.5)
            .rotate(std::f32::consts::FRAC_PI_4, Vec3::Z);
        // 0.5s wait -> fade in -> 0.5s wait
        timeline.forward(0.5);
        let polygon = timeline.play(fade_in(&polygon));
        timeline.forward(0.5);

        // let polygon = timeline.play(fade_in(&polygon));
        // timeline.hide(&polygon);
        // let mut svg = SvgItem::from_svg(SVG);
        // svg.scale(Vec3::splat(2.0));
        // let polygon = SvgItem::from(polygon.data);

        // let polygon = timeline.insert(polygon);
        // let svg = timeline.play(interpolate(polygon, svg));
        timeline.forward(0.5);

        // let mut arc = Arc {
        //     angle: std::f32::consts::PI / 2.0,
        //     radius: 300.0,
        // }
        // .build();
        // arc.set_color(Srgba::hex("58C4DDFF").unwrap())
        //     .set_stroke_width(20.0);

        // let arc = SvgItem::from(arc);
        // let arc = timeline.play(interpolate(svg, arc));
        // timeline.forward(0.5);

        // timeline.play(uncreate(arc));
        // timeline.forward(0.5);
    }
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=info")).init();

    MainScene.render(&AppOptions::default());
}
