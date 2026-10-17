use ranim::glam;
use std::{f64::consts::PI, sync::Arc};

use glam::dvec3;
use ranim::{
    anims::{
        creation::{CreationAnim, WritingAnim},
        fading::FadingAnim,
        morph::MorphAnim,
    },
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Circle, Polygon, Square},
    },
    prelude::*,
    utils::rate_functions::{linear, smooth},
};

#[allow(unused)]
fn pentagon() -> VItem {
    Polygon::new(
        (0..=5)
            .map(|i: i32| {
                let angle = i as f64 / 5.0 * 2.0 * PI;
                dvec3(angle.cos(), angle.sin(), 0.0) * 2.0
            })
            .collect(),
    )
    .with(|x| {
        x.set_color(manim::RED_C);
        x.with_origin(AabbPoint::CENTER, |x| {
            x.rotate_on_z(PI / 2.0);
        });
    })
    .into()
}

#[allow(unused)]
#[scene]
#[output]
fn fading(r: &mut RanimScene) {
    let mut pentagon_in = pentagon().with(|x| {
        x.move_to(dvec3(0.0, 2.0, 0.0));
    });
    let mut pentagon_out = pentagon().with(|x| {
        x.move_to(dvec3(0.0, -2.0, 0.0));
    });
    let duration = 1.0;
    r.play(CameraFrame::default().show().with_duration(duration));
    r.play(stack![
        pentagon_in.fade_in().with_rate_func(smooth),
        pentagon_out.fade_out().with_rate_func(smooth),
    ]);
}

#[allow(unused)]
#[scene]
fn creation(r: &mut RanimScene) {
    let mut pentagon_in = pentagon().with(|x| {
        x.move_to(dvec3(0.0, 2.0, 0.0));
    });
    let mut pentagon_out = pentagon().with(|x| {
        x.move_to(dvec3(0.0, -2.0, 0.0));
    });
    let duration = 1.0;
    r.play(CameraFrame::default().show().with_duration(duration));
    r.play(stack![
        pentagon_in.create().with_rate_func(smooth),
        pentagon_out.uncreate().with_rate_func(smooth),
    ]);
}

#[allow(unused)]
#[scene]
#[output]
fn writing(r: &mut RanimScene) {
    let mut pentagon_in = pentagon().with(|x| {
        x.move_to(dvec3(0.0, 2.0, 0.0));
    });
    let mut pentagon_out = pentagon().with(|x| {
        x.move_to(dvec3(0.0, -2.0, 0.0));
    });
    let duration = 1.0;
    r.play(CameraFrame::default().show().with_duration(duration));
    r.play(stack![
        pentagon_in.write().with_rate_func(smooth),
        pentagon_out.unwrite().with_rate_func(smooth),
    ]);
}

#[allow(unused)]
#[scene]
#[output]
fn transform(r: &mut RanimScene) {
    let src = Square::new(2.0).with(|x| {
        x.set_color(manim::RED_C).move_to(dvec3(0.0, 2.0, 0.0));
    });
    let dst = Circle::new(1.5).with(|x| {
        x.set_color(manim::BLUE_C).move_to(dvec3(0.0, -2.0, 0.0));
    });
    // dst.with_origin(AabbPoint::CENTER, |x| { x.rotate_on_z(PI / 4.0 + PI); }); // rotate to match src
    let mut item = VItem::from(src);
    let animation = item.morph_to(VItem::from(dst)).with_rate_func(linear);
    r.play(CameraFrame::default().show().with_duration(1.0));
    r.play(animation);
}

struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_scenes([
            fading::scene(),
            creation::scene(),
            writing::scene(),
            transform::scene(),
        ]);
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins(ScenesPlugin);
    let app = Arc::new(app);
    for name in ["fading", "creation", "writing", "transform"] {
        let scene = app.scene(name).expect("the scene is registered");
        ranim::cmd::render_scene(&scene, 3);
    }
}
