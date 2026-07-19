use ranim::glam;
use std::f64::consts::PI;

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
    utils::rate_functions::linear,
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
    let mut camera = AnimSequence::new();
    camera.push(CameraFrame::default().show()).hold(duration);
    r.play(camera);
    r.play(pentagon_in.fade_in());
    r.play(pentagon_out.fade_out());
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
    let mut camera = AnimSequence::new();
    camera.push(CameraFrame::default().show()).hold(duration);
    r.play(camera);
    r.play(pentagon_in.create());
    r.play(pentagon_out.uncreate());
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
    let mut camera = AnimSequence::new();
    camera.push(CameraFrame::default().show()).hold(duration);
    r.play(camera);
    r.play(pentagon_in.write());
    r.play(pentagon_out.unwrite());
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
    let mut camera = AnimSequence::new();
    camera.push(CameraFrame::default().show()).hold(1.0);
    r.play(camera);
    r.play(animation);
}

fn main() {
    ranim::render_scene!(fading);
    ranim::render_scene!(creation);
    ranim::render_scene!(writing);
    ranim::render_scene!(transform);
}
