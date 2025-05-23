use std::f64::consts::PI;

use glam::DVec3;
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
        let mut square = Square(2.0).build();
        square.set_color(manim::BLUE_C);

        let mut square = timeline.insert(square);
        let mut circle = Circle(2.0).build();
        circle.rotate(PI / 4.0 + PI, DVec3::Z);
        circle.set_color(manim::RED_C);

        timeline.play(square.transform_to(circle).apply()); // Use `apply` on an anim schedule to update rabject data
        timeline.play(square.unwrite()); // Do not use `apply` to keep the data in Rabject not changed
        timeline.play(square.write());
        timeline.play(square.fade_out());
    }
}

fn main() {
    #[cfg(feature = "app")]
    run_scene_app(HelloRanimScene);
    #[cfg(not(feature = "app"))]
    {
        render_scene(HelloRanimScene, &AppOptions::default());
        render_scene_at_sec(HelloRanimScene, 0.0, "preview.png", &AppOptions::default());
    }
}
