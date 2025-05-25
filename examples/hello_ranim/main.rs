use std::f64::consts::PI;

use log::LevelFilter;
use ranim::{
    animation::{creation::WritingAnim, fading::FadingAnim, transform::TransformAnim},
    color::palettes::manim,
    glam::DVec3,
    items::vitem::{
        VItem,
        geometry::{Circle, Square},
    },
    prelude::*,
    timeline::{TimelineTrait, TimelinesFunc},
};

#[scene]
struct HelloRanimScene;

impl TimelineConstructor for HelloRanimScene {
    fn construct(self, r: &mut RanimScene, _r_cam: TimelineId<CameraFrame>) {
        let square = Square::new(2.0).with(|square| {
            square.set_color(manim::BLUE_C);
        });
        let r_square = r.init_timeline(square);

        let circle = Circle::new(2.0).with(|circle| {
            circle
                .set_color(manim::RED_C)
                .rotate(PI / 4.0 + PI, DVec3::Z);
        });
        let r_vitem_circle = r.init_timeline(VItem::from(circle.clone()));

        let square = {
            let timeline = r.timeline_mut(&r_square);
            let square = timeline.play_with(|square| square.fade_in());
            timeline.hide();
            square
        };

        r.timelines_mut().sync();
        {
            let timeline = r.timeline_mut(&r_vitem_circle);
            timeline.play_with(|circle| VItem::from(square).transform_to(circle));
            timeline.forward(1.0);
            let circle = timeline.state().clone();
            timeline.play_with(|circle| circle.unwrite());
            timeline.play(circle.write());
            timeline.play_with(|circle| circle.fade_out());
        }
        r.timelines_mut().sync();
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
