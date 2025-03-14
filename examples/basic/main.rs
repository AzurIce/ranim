use std::f32;

use env_logger::Env;
use glam::{Vec3, vec3};
use ranim::animation::creation::{Color, CreationAnimSchedule, WritingAnimSchedule};
use ranim::animation::fading::FadingAnimSchedule;
use ranim::animation::transform::TransformAnimSchedule;
use ranim::color::palettes::manim;
use ranim::items::group::Group;
use ranim::items::svg_item::SvgItem;
use ranim::items::vitem::{Arc, Polygon, VItem};
use ranim::timeline::TimeMark;
use ranim::utils::rate_functions::linear;
use ranim::{prelude::*, typst_svg};

const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[scene]
struct BasicScene;

impl TimelineConstructor for BasicScene {
    fn construct<'t: 'r, 'r>(
        self,
        timeline: &'t RanimTimeline,
        _camera: &'r mut Rabject<'t, CameraFrame>,
    ) {
        timeline.forward(0.2);

        let mut svg = SvgItem::from_svg(SVG);
        svg.scale(Vec3::splat(2.0)).shift(vec3(0.0, 200.0, 0.0));
        let mut svg = timeline.insert(svg);
        let mut text = Group::<VItem>::from_svg(&typst_svg!(
            r#"
            #align(center)[
                #text(60pt)[Ranim]

                #text(20pt)[Hello 你好]
            ]
            "#
        ));
        text.iter_mut().for_each(|item| {
            item.set_fill_opacity(0.8).shift(Vec3::NEG_Y * 200.0);
        });
        let mut text = text
            .into_iter()
            .map(|item| timeline.insert(item))
            .collect::<Group<_>>();
        let len = text.len() as f32;
        let dur = 3.0 / (1.0 + (len - 1.0) * 0.2);
        // println!("{len}, {dur}");

        timeline.play_group(text.lagged_anim(0.2, |item| {
            item.write().with_duration(dur).with_rate_func(linear)
        }));
        timeline.play(svg.fade_in().with_duration(3.0)); // At the same time, the svg fade in
        timeline.sync();
        timeline.insert_time_mark(
            timeline.duration_secs(),
            TimeMark::Capture("preview.png".to_string()),
        );

        timeline.forward(0.5);
        timeline.play_group(text.lagged_anim(0.2, |item| {
            item.unwrite().with_duration(dur).with_rate_func(linear)
        }));
        timeline.play(svg.fade_out().with_duration(3.0));
        timeline.sync();

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
        timeline.play(polygon.fade_in()).sync();
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
        timeline.play(arc.transform_from(polygon_data)).sync();
        timeline.forward(0.5);

        // [svg] fade_out -> 0.5s wait
        timeline.play(arc.uncreate()).sync();
        timeline.forward(0.5);
    }
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=info,ranim=info")).init();

    render_timeline(BasicScene, &AppOptions::default());
}
