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
};

#[scene]
struct HelloRanimScene;

impl SceneConstructor for HelloRanimScene {
    fn construct(&self, r: &mut RanimScene, _r_cam: ItemId<CameraFrame>) {
        let square = Square::new(2.0).with(|square| {
            square.set_color(manim::BLUE_C);
        });

        let r_square = r.insert(square);
        {
            let timeline = r.timeline_mut(&r_square);
            timeline.play_with(|square| square.fade_in());
        };

        let circle = Circle::new(2.0).with(|circle| {
            circle
                .set_color(manim::RED_C)
                .rotate(PI / 4.0 + PI, DVec3::Z);
        });

        let r_vitem = r.map(r_square, VItem::from);
        {
            let timeline = r.timeline_mut(&r_vitem);
            timeline.play_with(|state| state.transform_to(circle.into()));
            timeline.forward(1.0);
            let circle = timeline.state().clone();
            timeline.play_with(|circle| circle.unwrite());
            timeline.play(circle.write());
            timeline.play_with(|circle| circle.fade_out());
        };
    }
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        #[cfg(debug_assertions)]
        pretty_env_logger::formatted_timed_builder()
            .filter(Some("ranim"), LevelFilter::Trace)
            .init();
        #[cfg(not(debug_assertions))]
        pretty_env_logger::formatted_timed_builder()
            .filter(Some("ranim"), LevelFilter::Info)
            .init();
    }

    #[cfg(feature = "app")]
    run_scene_app(HelloRanimScene);
    #[cfg(not(feature = "app"))]
    {
        render_scene(HelloRanimScene, &AppOptions::default());
        render_scene_at_sec(HelloRanimScene, 0.0, "preview.png", &AppOptions::default());
    }
}
