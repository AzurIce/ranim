#[cfg(target_arch = "wasm32")]
use ranim_app as _;

use ranim::{
    anims::morph::MorphAnim,
    color,
    color::palettes::manim,
    glam::DVec3,
    items::vitem::{VItem, geometry::Square},
    prelude::*,
    utils::rate_functions::linear,
};
use ranim_core::animation::StaticAnim;

#[scene]
#[output(dir = "perspective_blend")]
fn perspective_blend(r: &mut RanimScene) {
    let mut cam = CameraFrame::default();
    let r_cam = r.insert(cam.clone());

    // Update camera's state and show the new state
    cam.pos = DVec3::Z * 5.0;
    r.timeline_mut(r_cam).play(cam.show());

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
    .map(|color| {
        let face = square_with_color(color);
        (r.insert(face.clone()), face)
    });

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

    square_faces
        .iter_mut()
        .zip(transform_fns)
        .for_each(|((r_face, face), transform_fn)| {
            r.timeline_mut(*r_face)
                .play(face.morph(transform_fn).with_rate_func(linear))
                .hide();
        });

    let faces = square_faces.map(|(_, face)| face);
    let mut faces = faces.to_vec();

    let r_faces = r.insert_empty();
    r.timelines_mut().sync(); // TODO: make this better
    r.timeline_mut(r_faces).play(
        faces
            .morph(|data| {
                data.with_origin(AabbPoint::CENTER, |x| {
                    x.rotate_on_y(std::f64::consts::PI / 6.0);
                });
                data.with_origin(AabbPoint::CENTER, |x| {
                    x.rotate_on_x(std::f64::consts::PI / 6.0);
                });
            })
            .with_duration(4.0),
    );

    r.timeline_mut(r_cam).forward(2.0).play(
        cam.morph(|data| {
            data.perspective_blend = 1.0;
        })
        .with_duration(2.0),
    );
    r.insert_time_mark(
        r.timelines().max_total_secs(),
        TimeMark::Capture("preview.png".to_string()),
    );
}
