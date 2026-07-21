use ranim::{
    anims::morph::MorphAnim,
    color,
    color::palettes::manim,
    glam::DVec3,
    items::vitem::{VItem, geometry::Square},
    prelude::*,
    utils::rate_functions::{linear, smooth},
};
use ranim_core::animation::StaticAnim;

#[scene]
#[output(dir = "./output/perspective_blend")]
fn perspective_blend(r: &mut RanimScene) {
    let mut cam = CameraFrame {
        pos: DVec3::Z * 5.0,
        ..Default::default()
    };

    // Create a cube
    let side_length = 4.0;

    let square_with_color = |color: color::AlphaColor<color::Srgb>| {
        VItem::from(Square::new(side_length).with(|square| {
            square.set_color(color).set_fill_opacity(0.5);
        }))
    };

    // bottom, right, back, top, front, left
    let mut square_faces = [
        manim::TEAL_C,
        manim::GREEN_C,
        manim::BLUE_C,
        manim::PURPLE_C,
        manim::RED_C,
        manim::YELLOW_C,
    ]
    .map(square_with_color);

    let frac = 2.0;
    let transform_fns: [&dyn Fn(&mut VItem); 6] = [
        &(|data| {
            data.shift(DVec3::NEG_Y * side_length / frac);
            data.with_origin(AabbPoint::CENTER, |x| {
                x.rotate_on_x(std::f64::consts::PI / 2.0);
            });
        }),
        &(|data| {
            data.shift(DVec3::X * side_length / frac);
            data.with_origin(AabbPoint::CENTER, |x| {
                x.rotate_on_y(std::f64::consts::PI / 2.0);
            });
        }),
        &(|data| {
            data.shift(DVec3::NEG_Z * side_length / frac);
        }),
        &(|data| {
            data.shift(DVec3::Y * side_length / frac);
            data.with_origin(AabbPoint::CENTER, |x| {
                x.rotate_on_x(-std::f64::consts::PI / 2.0);
            });
        }),
        &(|data| {
            data.shift(DVec3::Z * side_length / frac);
        }),
        &(|data| {
            data.shift(DVec3::NEG_X * side_length / frac);
            data.with_origin(AabbPoint::CENTER, |x| {
                x.rotate_on_y(-std::f64::consts::PI / 2.0);
            });
        }),
    ];

    let mut content = AnimStack::new();
    square_faces
        .iter_mut()
        .zip(transform_fns)
        .for_each(|(face, transform_fn)| {
            content.push(face.morph(transform_fn).with_rate_func(linear));
        });

    let mut faces = square_faces.to_vec();

    let mut faces_sequence = AnimSequence::new();
    faces_sequence.forward(1.0).push(
        faces
            .morph(|data| {
                data.with_origin(AabbPoint::CENTER, |x| {
                    x.rotate_on_y(std::f64::consts::PI / 6.0);
                });
                data.with_origin(AabbPoint::CENTER, |x| {
                    x.rotate_on_x(std::f64::consts::PI / 6.0);
                });
            })
            .with_duration(4.0)
            .with_rate_func(smooth),
    );
    content.push(faces_sequence);

    let total_secs = content.duration_secs();
    let mut camera = AnimSequence::new();
    camera
        .push(cam.show())
        .hold(2.0)
        .push(
            cam.morph(|data| {
                data.perspective_blend = 1.0;
            })
            .with_duration(2.0)
            .with_rate_func(smooth),
        )
        .hold_to(total_secs);
    content.push(camera);
    r.play(content);
    r.insert_time_mark(total_secs, TimeMark::Capture("preview.png".to_string()));
}
