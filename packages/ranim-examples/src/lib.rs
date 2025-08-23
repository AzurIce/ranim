//! This crate contains all ranim examples

use std::f64::consts::PI;

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
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub mod tutorial {
    pub mod getting_started;
}
pub mod sort;

#[scene]
#[preview]
#[output]
/// Hello Ranim!
///
/// <canvas id="ranim-app-hello_ranim" width="1280" height="720" style="width: 100%;"></canvas>
/// <script type="module">
///   const { run_hello_ranim } = await ranim_examples;
///   run_hello_ranim();
/// </script>
pub fn hello_ranim(r: &mut RanimScene) {
    let _r_cam = r.insert_and_show(CameraFrame::default());
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
            .rotate(-PI / 4.0 + PI, DVec3::Z);
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

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn run_hello_ranim() {
    run_scene_app(
        hello_ranim_scene.constructor,
        hello_ranim_scene.name.to_string(),
    );
}
