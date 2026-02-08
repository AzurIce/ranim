use ranim::prelude::*;

pub mod test_scenes {
    use itertools::Itertools;
    use ranim::{
        anims::transform::TransformAnim,
        glam::{DVec3, dvec3},
        items::vitem::{
            VItem,
            geometry::{Circle, Square},
        },
    };

    use super::*;

    pub fn static_squares(r: &mut RanimScene, n: usize) {
        let _r_cam = r.insert(CameraFrame::default());

        let buff = 0.1;
        let size = 8.0 / n as f64;

        let unit = size + buff;
        let start = dvec3(-4.0, -4.0, 0.0);
        let _squares = (0..n)
            .cartesian_product(0..n)
            .map(|(i, j)| {
                Square::new(size).with(|square| {
                    square.move_to(start + unit * DVec3::X * j as f64 + unit * DVec3::Y * i as f64);
                })
            })
            .map(|item| r.insert(item))
            .collect::<Vec<_>>();
        r.timelines_mut().forward(1.0);
    }

    pub fn transform_squares(r: &mut RanimScene, n: usize) {
        let _r_cam = r.insert(CameraFrame::default());

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
            .map(|item| (r.insert(item.clone()), item))
            .collect::<Vec<_>>();
        let circles = (0..n)
            .cartesian_product(0..n)
            .map(|(i, j)| {
                VItem::from(Circle::new(size / 2.0).with(|circle| {
                    circle.move_to(start + unit * DVec3::X * j as f64 + unit * DVec3::Y * i as f64);
                }))
            })
            .collect::<Vec<_>>();
        squares
            .into_iter()
            .zip(circles)
            .for_each(|((r_square, item), circle)| {
                r.timeline_mut(r_square)
                    .play(item.clone().transform_to(circle));
            });
    }
}
