use glam::DVec3;
use ranim::{
    animation::transform::{GroupTransformAnimSchedule, TransformAnimSchedule},
    color::palettes::manim,
    components::Transformable,
    items::{group::Group, vitem::Square},
    prelude::*,
    timeline::TimeMark,
};

#[scene]
struct PerspectiveBlendScene;

impl TimelineConstructor for PerspectiveBlendScene {
    fn construct(self, timeline: &RanimTimeline, camera: &mut Rabject<CameraFrame>) {
        camera.data.pos = DVec3::Z * 5.0;
        timeline.update(camera);

        // Create a cube
        let side_length = 2.0;

        let mut anims = Vec::new();

        // Bottom face
        let mut bottom = Square(side_length).build();
        bottom.set_color(manim::TEAL_C).set_fill_opacity(0.5);
        let mut bottom = timeline.insert(bottom);
        anims.push(bottom.transform(|data| {
            data.shift(DVec3::NEG_Y * side_length / 2.0)
                .rotate(std::f64::consts::PI / 2.0, DVec3::X);
        }));

        // Right face
        let mut right = Square(side_length).build();
        right.set_color(manim::GREEN_C).set_fill_opacity(0.5);
        let mut right = timeline.insert(right);
        anims.push(right.transform(|data| {
            data.shift(DVec3::X * side_length / 2.0)
                .rotate(std::f64::consts::PI / 2.0, DVec3::Y);
        }));

        // Back face
        let mut back = Square(side_length).build();
        back.set_color(manim::BLUE_C).set_fill_opacity(0.5);
        let mut back = timeline.insert(back);
        anims.push(back.transform(|data| {
            data.shift(DVec3::NEG_Z * side_length / 2.0);
        }));

        // Top face
        let mut top = Square(side_length).build();
        top.set_color(manim::PURPLE_C).set_fill_opacity(0.5);
        let mut top = timeline.insert(top);
        anims.push(top.transform(|data| {
            data.shift(DVec3::Y * side_length / 2.0)
                .rotate(-std::f64::consts::PI / 2.0, DVec3::X);
        }));

        // Front face (facing camera)
        let mut front = Square(side_length).build();
        front.set_color(manim::RED_C).set_fill_opacity(0.5);
        let mut front = timeline.insert(front);

        anims.push(front.transform(|data| {
            data.shift(DVec3::Z * side_length / 2.0);
        }));

        // Left face
        let mut left = Square(side_length).build();
        left.set_color(manim::YELLOW_C).set_fill_opacity(0.5);
        let mut left = timeline.insert(left);
        anims.push(left.transform(|data| {
            data.shift(DVec3::NEG_X * side_length / 2.0)
                .rotate(-std::f64::consts::PI / 2.0, DVec3::Y);
        }));

        timeline.play(Group(anims).apply()).sync();

        let mut cube = Group(vec![front, back, right, left, top, bottom]);

        timeline.play(
            cube.transform(|data| {
                data.rotate(std::f64::consts::PI / 6.0, DVec3::Y)
                    .rotate(std::f64::consts::PI / 6.0, DVec3::X);
            })
            .with_duration(4.0),
        );

        timeline.play(
            camera
                .transform(|data| {
                    data.perspective_blend = 1.0;
                })
                .with_duration(2.0)
                .with_padding(2.0, 0.0),
        );
        timeline.sync();
        timeline.insert_time_mark(
            timeline.duration_secs(),
            TimeMark::Capture("preview.png".to_string()),
        );
    }
}

fn main() {
    let options = AppOptions {
        pixel_size: (1280, 720),
        frame_rate: 60,
        ..Default::default()
    };

    render_scene(PerspectiveBlendScene, &options);
}
