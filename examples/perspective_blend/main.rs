use glam::{Vec3, vec3};
use ranim::{
    animation::transform::TransformAnimSchedule, color::palettes::manim, components::Transformable,
    items::group::Group, items::vitem::Square, prelude::*,
};

#[scene]
struct PerspectiveBlendScene;

impl TimelineConstructor for PerspectiveBlendScene {
    fn construct<'t: 'r, 'r>(
        self,
        timeline: &'t RanimTimeline,
        camera: &'r mut Rabject<'t, CameraFrame>,
    ) {
        // Create a cube
        let side_length = 2.0;

        // Front face (facing camera)
        let mut front = Square(side_length).build();
        front
            .set_color(manim::RED_C)
            .shift(vec3(0.0, 0.0, -side_length / 2.0));

        // Back face
        let mut back = Square(side_length).build();
        back.set_color(manim::BLUE_C)
            .shift(vec3(0.0, 0.0, side_length / 2.0))
            .rotate(std::f32::consts::PI, Vec3::Y);

        // Right face
        let mut right = Square(side_length).build();
        right
            .set_color(manim::GREEN_C)
            .shift(vec3(side_length / 2.0, 0.0, 0.0))
            .rotate(std::f32::consts::PI / 2.0, Vec3::Y);

        // Left face
        let mut left = Square(side_length).build();
        left.set_color(manim::YELLOW_C)
            .shift(vec3(-side_length / 2.0, 0.0, 0.0))
            .rotate(-std::f32::consts::PI / 2.0, Vec3::Y);

        // Top face
        let mut top = Square(side_length).build();
        top.set_color(manim::PURPLE_C)
            .shift(vec3(0.0, side_length / 2.0, 0.0))
            .rotate(-std::f32::consts::PI / 2.0, Vec3::X);

        // Bottom face
        let mut bottom = Square(side_length).build();
        bottom
            .set_color(manim::TEAL_C)
            .shift(vec3(0.0, -side_length / 2.0, 0.0))
            .rotate(std::f32::consts::PI / 2.0, Vec3::X);

        let mut cube = Group(vec![front, back, right, left, top, bottom]);

        cube.rotate(std::f32::consts::PI / 6.0, Vec3::Y);
        cube.rotate(std::f32::consts::PI / 6.0, Vec3::X);
        // Position cube and camera
        let mut _cubes = timeline.insert_group(cube);

        // Move camera to show the cube
        camera.data.pos = vec3(0.0, 0.0, 5.0);
        timeline.update(&camera);

        timeline.forward(1.0);

        timeline.play(camera.transform(|data| {
            data.perspective_blend = 1.0;
        }).with_duration(3.0));
        timeline.sync();
    }
}

fn main() {
    let options = AppOptions {
        pixel_size: (1280, 720),
        frame_rate: 60,
        ..Default::default()
    };

    render_timeline(PerspectiveBlendScene, &options);
}
