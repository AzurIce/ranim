use std::f64::consts::PI;

use glam::DVec3;
use log::LevelFilter;
use ranim::{
    animation::{creation::WritingAnim, fading::FadingAnim, transform::TransformAnim},
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Circle, Square},
    },
    prelude::*,
};

#[scene]
struct HelloRanimScene;

impl TimelineConstructor for HelloRanimScene {
    fn construct(self, timeline: &RanimTimeline, _camera: PinnedItem<CameraFrame>) {
        let mut square = Square::new(2.0).with(|square| {
            square.fill_rgba = manim::BLUE_C;
            square.stroke_rgba = manim::BLUE_C;
        });

        timeline.play(square.clone().fade_in());
        let mut circle = Circle::new(2.0).with(|circle| {
            circle.fill_rgba = manim::RED_C;
            circle.stroke_rgba = manim::RED_C;
            circle.rotate(PI / 4.0 + PI, DVec3::Z);
        });

        timeline.play(VItem::from(square).transform_to(circle.clone()));
        timeline.sync();

        let circle = timeline.pin(circle);
        timeline.forward(1.0);
        let circle = timeline.unpin(circle);
        timeline.play(VItem::from(circle.clone()).unwrite());
        timeline.play(VItem::from(circle.clone()).write());
        timeline.play(circle.fade_out());
    }
}

fn main() {
    #[cfg(debug_assertions)]
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("ranim"), LevelFilter::Trace)
        .init();
    #[cfg(feature = "app")]
    run_scene_app(HelloRanimScene);
    #[cfg(not(feature = "app"))]
    {
        render_scene(HelloRanimScene, &AppOptions::default());
        render_scene_at_sec(HelloRanimScene, 0.0, "preview.png", &AppOptions::default());
    }
}
