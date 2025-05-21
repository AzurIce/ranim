use ranim::{
    animation::{
        GroupAnimFunction,
        transform::{GroupTransformAnim, TransformAnim},
    },
    color::palettes::manim,
    glam::DVec3,
    items::vitem::{VItem, geometry::Square},
    prelude::*,
    timeline::TimeMark,
    utils::rate_functions::linear,
};

#[scene]
struct PerspectiveBlendScene;

impl TimelineConstructor for PerspectiveBlendScene {
    fn construct(self, timeline: &RanimTimeline, camera: PinnedItem<CameraFrame>) {
        let mut camera = timeline.unpin(camera);
        camera.pos = DVec3::Z * 5.0;
        let camera = timeline.pin(camera);

        // Create a cube
        let side_length = 2.0;

        let square_with_color = |color: color::AlphaColor<color::Srgb>| {
            VItem::from(Square::new(side_length).with(|square| {
                square.set_color(color).set_fill_opacity(0.5);
            }))
        };

        let bottom = square_with_color(manim::TEAL_C);
        let right = square_with_color(manim::GREEN_C);
        let back = square_with_color(manim::BLUE_C);
        let top = square_with_color(manim::PURPLE_C);
        let front = square_with_color(manim::RED_C);
        let left = square_with_color(manim::YELLOW_C);

        let bottom = bottom.transform(|data| {
            data.shift(DVec3::NEG_Y * side_length / 2.0)
                .rotate(std::f64::consts::PI / 2.0, DVec3::X);
        });
        let right = right.transform(|data| {
            data.shift(DVec3::X * side_length / 2.0)
                .rotate(std::f64::consts::PI / 2.0, DVec3::Y);
        });
        let back = back.transform(|data| {
            data.shift(DVec3::NEG_Z * side_length / 2.0);
        });
        let top = top.transform(|data| {
            data.shift(DVec3::Y * side_length / 2.0)
                .rotate(-std::f64::consts::PI / 2.0, DVec3::X);
        });
        let front = front.transform(|data| {
            data.shift(DVec3::Z * side_length / 2.0);
        });
        let left = left.transform(|data| {
            data.shift(DVec3::NEG_X * side_length / 2.0)
                .rotate(-std::f64::consts::PI / 2.0, DVec3::Y);
        });

        let faces = timeline.play([bottom, right, back, top, front, left].with_rate_func(linear));

        timeline.schedule(
            faces
                .transform(|data| {
                    data.rotate(std::f64::consts::PI / 6.0, DVec3::Y)
                        .rotate(std::f64::consts::PI / 6.0, DVec3::X);
                })
                .with_duration(4.0),
        );

        timeline.forward(2.0);

        let camera = timeline.unpin(camera);
        timeline.play(
            camera
                .transform(|data| {
                    data.perspective_blend = 1.0;
                })
                .with_duration(2.0),
        );
        timeline.insert_time_mark(
            timeline.cur_sec(),
            TimeMark::Capture("preview.png".to_string()),
        );
    }
}

fn main() {
    #[cfg(not(feature = "app"))]
    {
        let options = AppOptions {
            pixel_size: (1280, 720),
            frame_rate: 60,
            ..Default::default()
        };
        render_scene(PerspectiveBlendScene, &options);
    }

    // reuires "app" feature
    #[cfg(feature = "app")]
    run_scene_app(PerspectiveBlendScene);
}
