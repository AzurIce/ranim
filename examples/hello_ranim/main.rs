use std::f64::consts::PI;

use glam::DVec3;
use log::LevelFilter;
use ranim::{
    animation::{
        creation::WritingAnimSchedule, fading::FadingAnimSchedule, transform::TransformAnimSchedule,
    },
    color::palettes::manim,
    items::vitem::{Circle, Square},
    prelude::*,
};

#[scene]
struct HelloRanimScene;

impl TimelineConstructor for HelloRanimScene {
    fn construct(self, timeline: &RanimTimeline, _camera: &mut Rabject<CameraFrame>) {
        let mut square = Rabject::new(Square(2.0).build());
        square.set_color(manim::BLUE_C);

        timeline.insert(&square);
        let mut circle = Circle(2.0).build();
        circle.rotate(PI / 4.0 + PI, DVec3::Z);
        circle.set_color(manim::RED_C);

        let circle = timeline.play(square.clone().transform_to(circle)); // Use `apply` on an anim schedule to update rabject data
        let circle = Rabject::new(circle);
        timeline.hide(&square);
        timeline.insert(&circle);
        timeline.play(circle.clone().unwrite()); // Do not use `apply` to keep the data in Rabject not changed
        timeline.play(circle.clone().write());
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
