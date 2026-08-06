use ranim::prelude::*;

pub mod test_scenes {
    use itertools::Itertools;
    use ranim::{
        anims::pure::morph::MorphAnim,
        core::animation::{AnimStack, StaticAnim},
        glam::{DVec3, dvec3},
        items::vitem::{
            VItem,
            geometry::{Circle, Square},
        },
        utils::rate_functions::smooth,
    };

    use super::*;

    pub fn static_squares(r: &mut RanimScene, n: usize) {
        let buff = 0.1;
        let size = 8.0 / n as f64;

        let unit = size + buff;
        let start = dvec3(-4.0, -4.0, 0.0);
        let squares = (0..n)
            .cartesian_product(0..n)
            .map(|(i, j)| {
                Square::new(size).with(|square| {
                    square.move_to(start + unit * DVec3::X * j as f64 + unit * DVec3::Y * i as f64);
                })
            })
            .collect::<Vec<_>>();

        let mut content = AnimStack::new();
        for square in squares {
            content.push(square.show().with_duration(1.0));
        }
        content.push(CameraFrame::default().show().with_duration(1.0));
        r.play(content);
    }

    pub fn transform_squares(r: &mut RanimScene, n: usize) {
        let buff = 0.1;
        let size = 8.0 / n as f64 - buff;

        let unit = size + buff;
        let start = dvec3(-4.0, -4.0, 0.0);
        let squares = (0..n)
            .cartesian_product(0..n)
            .map(|(i, j)| {
                VItem::from(Square::new(size).with(|square| {
                    square.move_to(start + unit * DVec3::X * j as f64 + unit * DVec3::Y * i as f64);
                }))
            })
            .collect::<Vec<_>>();
        let circles = (0..n)
            .cartesian_product(0..n)
            .map(|(i, j)| {
                VItem::from(Circle::new(size / 2.0).with(|circle| {
                    circle.move_to(start + unit * DVec3::X * j as f64 + unit * DVec3::Y * i as f64);
                }))
            })
            .collect::<Vec<_>>();
        let mut content = AnimStack::new();
        for (mut square, circle) in squares.into_iter().zip(circles) {
            content.push(square.morph_to(circle).with_rate_func(smooth));
        }
        content.push(CameraFrame::default().show().with_duration(1.0));
        r.play(content);
    }
}
