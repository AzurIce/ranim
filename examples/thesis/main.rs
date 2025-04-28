use std::f64::consts::PI;

use glam::{DVec3, dvec3};
use ranim::{
    animation::{
        creation::{CreationAnimSchedule, WritingAnimSchedule},
        fading::FadingAnimSchedule,
        transform::TransformAnimSchedule,
    },
    color::palettes::manim,
    items::vitem::{Circle, Polygon, Square, VItem},
    prelude::*,
    utils::rate_functions::linear,
};

#[allow(unused)]
fn pentagon() -> VItem {
    let mut pentagon = Polygon(
        (0..=5)
            .map(|i| {
                let angle = i as f64 / 5.0 * 2.0 * PI;
                dvec3(angle.cos(), angle.sin(), 0.0) * 2.0
            })
            .collect(),
    )
    .build();
    pentagon.set_color(manim::RED_C).rotate(PI / 2.0, DVec3::Z);
    pentagon
}

#[allow(unused)]
#[scene]
struct FadingScene;

impl TimelineConstructor for FadingScene {
    fn construct(self, timeline: &RanimTimeline, _camera: &mut Rabject<CameraFrame>) {
        let mut pentagon_in = timeline.insert(pentagon());
        pentagon_in.data.put_center_on(dvec3(0.0, 2.0, 0.0));
        timeline.update(&pentagon_in);
        let mut pentagon_out = timeline.insert(pentagon());
        pentagon_out.data.put_center_on(dvec3(0.0, -2.0, 0.0));
        timeline.update(&pentagon_in);
        timeline.play(pentagon_in.fade_in().with_rate_func(linear));
        timeline.play(pentagon_out.fade_out().with_rate_func(linear));
        timeline.sync();
    }
}

#[allow(unused)]
#[scene]
struct CreationScene;

impl TimelineConstructor for CreationScene {
    fn construct(self, timeline: &RanimTimeline, _camera: &mut Rabject<CameraFrame>) {
        let mut pentagon_in = timeline.insert(pentagon());
        pentagon_in.data.put_center_on(dvec3(0.0, 2.0, 0.0));
        timeline.update(&pentagon_in);
        let mut pentagon_out = timeline.insert(pentagon());
        pentagon_out.data.put_center_on(dvec3(0.0, -2.0, 0.0));
        timeline.update(&pentagon_in);
        timeline.play(pentagon_in.create().with_rate_func(linear));
        timeline.play(pentagon_out.uncreate().with_rate_func(linear));
        timeline.sync();
    }
}

#[allow(unused)]
#[scene]
struct WritingScene;

impl TimelineConstructor for WritingScene {
    fn construct(self, timeline: &RanimTimeline, _camera: &mut Rabject<CameraFrame>) {
        let mut pentagon_in = timeline.insert(pentagon());
        pentagon_in.data.put_center_on(dvec3(0.0, 2.0, 0.0));
        timeline.update(&pentagon_in);
        let mut pentagon_out = timeline.insert(pentagon());
        pentagon_out.data.put_center_on(dvec3(0.0, -2.0, 0.0));
        timeline.update(&pentagon_in);
        timeline.play(pentagon_in.write().with_rate_func(linear));
        timeline.play(pentagon_out.unwrite().with_rate_func(linear));
        timeline.sync();
    }
}

#[allow(unused)]
#[scene]
struct TransformScene;

impl TimelineConstructor for TransformScene {
    fn construct(self, timeline: &RanimTimeline, _camera: &mut Rabject<CameraFrame>) {
        let mut src = timeline.insert(Square(2.0).build());
        src.data.set_color(manim::RED_C);
        src.data.put_center_on(dvec3(0.0, 2.0, 0.0));
        timeline.update(&src);
        let mut dst = Circle(1.5).build();
        // dst.rotate(PI / 4.0 + PI, DVec3::Z); // rotate to match src
        dst.set_color(manim::BLUE_C);
        dst.put_center_on(dvec3(0.0, -2.0, 0.0));
        timeline.play(src.transform_to(dst).with_rate_func(linear));
        timeline.sync();
    }
}

fn main() {
    let options = AppOptions {
        pixel_size: (1080, 1080),
        frame_rate: 5,
        save_frames: true,
        output_dir: "output-thesis",
        ..Default::default()
    };
    // render_scene(FadingScene, &options);
    // render_scene(CreationScene, &options);
    // render_scene(WritingScene, &options);
    render_scene(TransformScene, &options);
}
