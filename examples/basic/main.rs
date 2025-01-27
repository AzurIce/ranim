use std::time::{Duration, Instant};

use bevy_color::Srgba;
use env_logger::Env;
use glam::{vec3, Vec3};
use log::info;
use ranim::animation::creation::{self, Color};
use ranim::animation::fading::{self, Fading};
use ranim::components::TransformAnchor;
use ranim::glam::vec2;

use ranim::animation::interpolate::Interpolate;
use ranim::items::vitem::{Arc, Polygon};
use ranim::{prelude::*, typst_svg, Scene, SceneDesc};

const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

struct MainScene;

impl Scene for MainScene {
    fn desc() -> ranim::SceneDesc {
        SceneDesc {
            name: "basic".to_string(),
        }
    }
    fn construct<T: ranim::RanimApp>(&mut self, app: &mut T) {
        let t = Instant::now();
        info!("running...");

        // let mut ranim_text = Svg::from_svg(&typst_svg!("#text(60pt)[Ranim]")).build();
        // ranim_text.shift(-ranim_text.bounding_box().center());

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
            .set_opacity(0.5);
        polygon.vpoints.rotate(std::f32::consts::FRAC_PI_4, Vec3::Z, TransformAnchor::origin());

        // 0.5s wait -> fade in -> 0.5s wait
        app.wait(Duration::from_secs_f32(0.5));
        let polygon = app.play(polygon, fading::fade_in());
        app.wait(Duration::from_secs_f32(0.5));

        // let mut svg = Svg::from_svg(SVG).build();
        // svg.shift(center);

        // info!("polygon transform to svg");
        // let svg = app.play_in_canvas(&canvas, polygon, Interpolate::new(svg.clone()));
        // app.wait(Duration::from_secs_f32(0.5));

        let mut arc = Arc {
            angle: std::f32::consts::PI / 2.0,
            radius: 300.0,
        }.build();
        arc.set_color(Srgba::hex("58C4DDFF").unwrap()).set_stroke_width(40.0);

        info!("polygon transform to arc");
        let arc = app.play(polygon, Interpolate::new(arc.clone()));
        app.wait(Duration::from_secs_f32(0.5));

        info!("arc uncreate");
        app.play_remove(arc, creation::uncreate());
        app.wait(Duration::from_secs_f32(0.5));

        info!(
            "Rendered {} frames({}s) in {:?}",
            app.frame_cnt(),
            app.frame_time(),
            t.elapsed()
        );
    }
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=info")).init();

    MainScene.render();
}
