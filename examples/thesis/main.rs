use std::f64::consts::PI;

use glam::{DVec3, dvec3};
use ranim::{
    animation::{
        creation::{CreationAnim, WritingAnim},
        fading::FadingAnim,
        transform::TransformAnim,
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
            .map(|i| {
                let angle = i as f64 / 5.0 * 2.0 * PI;
                dvec3(angle.cos(), angle.sin(), 0.0) * 2.0
            })
            .collect(),
    )
    .with(|x| {
        x.set_color(manim::RED_C).rotate(PI / 2.0, DVec3::Z);
    })
    .into()
}

#[allow(unused)]
#[scene]
#[output]
fn fading(r: &mut RanimScene) {
    let _r_cam = r.insert_and_show(CameraFrame::default());
    let pentagon_in = pentagon().with(|x| {
        x.put_center_on(dvec3(0.0, 2.0, 0.0));
    });
    let pentagon_out = pentagon().with(|x| {
        x.put_center_on(dvec3(0.0, -2.0, 0.0));
    });
    let r_in = r.insert(pentagon_in);
    let r_out = r.insert(pentagon_out);
    r.timeline_mut(&r_in).play_with(|item| item.fade_in());
    r.timeline_mut(&r_out).play_with(|item| item.fade_out());
}

#[allow(unused)]
#[scene]
fn creation(r: &mut RanimScene) {
    let _r_cam = r.insert_and_show(CameraFrame::default());

    let pentagon_in = pentagon().with(|x| {
        x.put_center_on(dvec3(0.0, 2.0, 0.0));
    });
    let pentagon_out = pentagon().with(|x| {
        x.put_center_on(dvec3(0.0, -2.0, 0.0));
    });
    let r_in = r.insert(pentagon_in);
    let r_out = r.insert(pentagon_out);
    r.timeline_mut(&r_in).play_with(|item| item.create());
    r.timeline_mut(&r_out).play_with(|item| item.uncreate());
}

#[allow(unused)]
#[scene]
#[output]
fn writing(r: &mut RanimScene) {
    let _r_cam = r.insert_and_show(CameraFrame::default());
    let pentagon_in = pentagon().with(|x| {
        x.put_center_on(dvec3(0.0, 2.0, 0.0));
    });
    let pentagon_out = pentagon().with(|x| {
        x.put_center_on(dvec3(0.0, -2.0, 0.0));
    });
    let r_in = r.insert(pentagon_in);
    let r_out = r.insert(pentagon_out);
    r.timeline_mut(&r_in).play_with(|item| item.write());
    r.timeline_mut(&r_out).play_with(|item| item.unwrite());
}

#[allow(unused)]
#[scene]
#[output]
fn transform(r: &mut RanimScene) {
    let _r_cam = r.insert_and_show(CameraFrame::default());
    let src = Square::new(2.0).with(|x| {
        x.set_color(manim::RED_C)
            .put_center_on(dvec3(0.0, 2.0, 0.0));
    });
    let dst = Circle::new(1.5).with(|x| {
        x.set_color(manim::BLUE_C)
            .put_center_on(dvec3(0.0, -2.0, 0.0));
    });
    // dst.rotate(PI / 4.0 + PI, DVec3::Z); // rotate to match src
    let r_item = r.insert(VItem::from(src));
    r.timeline_mut(&r_item)
        .play_with(|item| item.transform_to(VItem::from(dst)).with_rate_func(linear));
}

fn main() {
    #[cfg(feature = "app")]
    {
        preview_scene(fading_scene);
        preview_scene(creation_scene);
        preview_scene(writing_scene);
        preview_scene(transform_scene);
    }
    #[cfg(not(feature = "app"))]
    {
        render_scene(fading_scene);
        render_scene(creation_scene);
        render_scene(writing_scene);
        render_scene(transform_scene);
    }
}
