use ranim::{
    animation::transform::TransformAnim,
    color::palettes::manim,
    glam::DVec3,
    items::{
        Group,
        vitem::{VItem, geometry::Square},
    },
    prelude::*,
    timeline::TimeMark,
    utils::rate_functions::linear,
};

#[scene]
#[preview]
#[output(dir = "perspective_blend")]
fn perspective_blend(r: &mut RanimScene) {
    let r_cam = r.insert_and_show(CameraFrame::default());
    r.timeline_mut(&r_cam).update_with(|cam| {
        cam.pos = DVec3::Z * 5.0;
    });

    // Create a cube
    let side_length = 2.0;

    let square_with_color = |color: color::AlphaColor<color::Srgb>| {
        VItem::from(Square::new(side_length).with(|square| {
            square.set_color(color).set_fill_opacity(0.5);
        }))
    };

    // bottom, right, back, top, front, left
    let square_faces = [
        manim::TEAL_C,
        manim::GREEN_C,
        manim::BLUE_C,
        manim::PURPLE_C,
        manim::RED_C,
        manim::YELLOW_C,
    ]
    .map(|color| r.insert(square_with_color(color)));

    let transform_fns: [&dyn Fn(&mut VItem); 6] = [
        &(|data| {
            data.shift(DVec3::NEG_Y * side_length / 2.0)
                .rotate(std::f64::consts::PI / 2.0, DVec3::X);
        }),
        &(|data| {
            data.shift(DVec3::X * side_length / 2.0)
                .rotate(std::f64::consts::PI / 2.0, DVec3::Y);
        }),
        &(|data| {
            data.shift(DVec3::NEG_Z * side_length / 2.0);
        }),
        &(|data| {
            data.shift(DVec3::Y * side_length / 2.0)
                .rotate(-std::f64::consts::PI / 2.0, DVec3::X);
        }),
        &(|data| {
            data.shift(DVec3::Z * side_length / 2.0);
        }),
        &(|data| {
            data.shift(DVec3::NEG_X * side_length / 2.0)
                .rotate(-std::f64::consts::PI / 2.0, DVec3::Y);
        }),
    ];

    let square_faces = square_faces
        .iter()
        .zip(transform_fns)
        .map(|(r_face, transform_fn)| {
            r.timeline_mut(r_face)
                .play_with(|face| face.transform(transform_fn).with_rate_func(linear))
                .hide()
                .state()
                .clone()
        })
        .collect::<Vec<_>>();

    let faces = Group(square_faces);
    let r_faces = r.insert(faces);
    r.timelines_mut().sync(); // TODO: make this better
    r.timeline_mut(&r_faces).play_with(|faces| {
        faces
            .transform(|data| {
                data.rotate(std::f64::consts::PI / 6.0, DVec3::Y)
                    .rotate(std::f64::consts::PI / 6.0, DVec3::X);
            })
            .with_duration(4.0)
    });

    r.timeline_mut(&r_cam).forward(2.0).play_with(|cam| {
        cam.transform(|data| {
            data.perspective_blend = 1.0;
        })
        .with_duration(2.0)
    });
    r.insert_time_mark(
        r.timelines().max_total_secs(),
        TimeMark::Capture("preview.png".to_string()),
    );
}

