use std::ops::{Deref, DerefMut};

use glam::DMat3;
use glam::DVec3;
use glam::dvec2;
use glam::dvec3;
use itertools::Itertools;
use ranim_macros::Interpolatable;

use crate::prelude::Partial;
use crate::traits::BoundingBox;
use crate::traits::PointsFunc;
use crate::traits::Position;
use crate::utils::bezier::trim_quad_bezier;
use crate::utils::math::interpolate_usize;

use super::Anchor;
use super::ComponentVec;

/// VPointComponentVec is used to represent a bunch of quad bezier paths.
///
/// Every 3 elements in the inner vector is a quad bezier path.
///
/// | 0(Anchor) | 1(Handle) | 2(Anchor) | 3(Handle) | 4(Anchor) |
/// |-----------|-----------|-----------|-----------|-----------|
/// | a | b | c | d | e(subpath0) |
///
/// If the handle is equal to the previous anchor, it represents a subpath's end.
///
/// | 0(Anchor) | 1(Handle) | 2(Anchor) | 3(Handle) | 4(Anchor) | 5(Handle) | 6(Anchor) |
/// |-----------|-----------|-----------|-----------|-----------|-----------|-----------|
/// | a | b | c | c(subpath0) | d | e | f (subpath1) |
#[derive(Debug, Clone, PartialEq, Interpolatable)]
pub struct VPointComponentVec(pub ComponentVec<DVec3>);

impl Deref for VPointComponentVec {
    type Target = ComponentVec<DVec3>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for VPointComponentVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl VPointComponentVec {
    pub fn get_seg(&self, idx: usize) -> Option<&[DVec3; 3]> {
        self.get(idx * 2..idx * 2 + 3)
            .and_then(|seg| seg.try_into().ok())
    }

    pub fn get_closepath_flags(&self) -> Vec<bool> {
        let len = self.len();
        let mut flags = vec![false; len];

        // println!("{:?}", self.0);
        let mut i = 0;
        for mut chunk in &self.0.iter().enumerate().skip(2).chunks(2) {
            let (a, b) = (chunk.next(), chunk.next());
            if let Some((ia, a)) = match (a, b) {
                (Some((ia, a)), Some((_ib, b))) => {
                    // println!("chunk[{ia}, {_ib}] {:?}", [a, b]);
                    if a == b { Some((ia, a)) } else { None }
                }
                (Some((ia, a)), None) => Some((ia, a)),
                _ => unreachable!(),
            } {
                // println!("### path end ###");
                if (a - self.get(i).unwrap()).length_squared() <= 0.0001 {
                    // println!("### path closed ###");
                    flags[i..=ia].fill(true);
                }
                i = ia + 2;
            }
        }

        // println!("{:?}", flags);

        flags
    }

    /// Put the start and end points of the item on the given points.
    pub fn put_start_and_end_on(&mut self, start: DVec3, end: DVec3) -> &mut Self {
        let (cur_start, cur_end) = (
            self.first().cloned().unwrap_or_default(),
            self.last().cloned().unwrap_or_default(),
        );
        let cur_v = cur_end - cur_start;
        if cur_v.length_squared() <= f64::EPSILON {
            return self;
        }

        let v = end - start;
        self.scale_by_anchor(
            DVec3::splat(v.length() / cur_v.length()),
            Anchor::Point(cur_start),
        );
        let angle = cur_v.y.atan2(-cur_v.x) - v.y.atan2(-v.x) + std::f64::consts::PI / 2.0;
        self.rotate(angle, DVec3::Z);
        let cur_xy = dvec2(cur_v.x, cur_v.y);
        let cur_xy = cur_xy * cur_xy.abs().normalize();

        let xy = dvec2(v.x, v.y);
        let xy = xy * xy.abs().normalize();
        let angle = cur_v.z.atan2(-cur_xy.length()) - v.z.atan2(-xy.length());
        self.rotate(angle, dvec3(-v.y, v.x, 0.0));
        let cur_start = self.first().cloned().unwrap();
        self.shift(start - cur_start);

        self
    }
}

impl Partial for VPointComponentVec {
    fn get_partial(&self, range: std::ops::Range<f64>) -> Self {
        let max_anchor_idx = self.len() / 2;

        let (start_index, start_residue) = interpolate_usize(0, max_anchor_idx, range.start);
        let (end_index, end_residue) = interpolate_usize(0, max_anchor_idx, range.end);

        if end_index - start_index == 0 {
            let seg = self.get_seg(start_index).unwrap().map(|p| p);
            let quad = trim_quad_bezier(&seg, start_residue, end_residue);
            VPointComponentVec(quad.into())
        } else {
            let mut partial = Vec::with_capacity((end_index - start_index + 1 + 2) * 2 + 1);

            let seg = self.get_seg(start_index).unwrap().map(|p| p);
            let start_part = trim_quad_bezier(&seg, start_residue, 1.0);
            partial.extend_from_slice(&start_part);

            // If start_index < end_index - 1, we need to add the middle segment
            //  start     mid    end
            // [o - o] [- o - o] [- o]
            if end_index - start_index > 1 {
                let mid = self
                    .get((start_index + 1) * 2 + 1..=end_index * 2)
                    .unwrap()
                    .iter();
                partial.extend(mid);
            }

            if end_residue != 0.0 {
                let seg = self.get_seg(end_index).unwrap().map(|p| p);
                let end_part = trim_quad_bezier(&seg, 0.0, end_residue);
                partial.extend_from_slice(&end_part[1..]);
            }

            VPointComponentVec(partial.into())
        }
    }
}

impl BoundingBox for [DVec3] {
    fn get_bounding_box(&self) -> [DVec3; 3] {
        let [min, max] = self
            .iter()
            .cloned()
            .map(|x| [x, x])
            .reduce(|[acc_min, acc_max], [min, max]| [acc_min.min(min), acc_max.max(max)])
            .unwrap_or([DVec3::ZERO, DVec3::ZERO]);
        [min, (min + max) / 2.0, max]
    }
}

pub fn wrap_point_func_with_anchor(
    f: impl Fn(&mut DVec3) + Copy,
    anchor: DVec3,
) -> impl Fn(&mut DVec3) + Copy {
    move |points| {
        *points -= anchor;
        f(points);
        *points += anchor;
    }
}

impl Position for [DVec3] {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.iter_mut().for_each(|p| *p += shift);
        self
    }

    fn rotate_by_anchor(&mut self, angle: f64, axis: DVec3, anchor: super::Anchor) -> &mut Self {
        let rotation = DMat3::from_axis_angle(axis, angle);
        let p = match anchor {
            Anchor::Point(point) => point,
            Anchor::Edge(edge) => self.get_bounding_box_point(edge),
        };
        self.iter_mut()
            .for_each(wrap_point_func_with_anchor(|p| *p = rotation * *p, p));
        self
    }

    fn scale_by_anchor(&mut self, scale: DVec3, anchor: super::Anchor) -> &mut Self {
        let p = match anchor {
            Anchor::Point(point) => point,
            Anchor::Edge(edge) => self.get_bounding_box_point(edge),
        };
        self.iter_mut()
            .for_each(wrap_point_func_with_anchor(|p| *p *= scale, p));
        self
    }
}

impl PointsFunc for [DVec3] {
    fn apply_points_func(&mut self, f: impl for<'a> Fn(&'a mut [DVec3])) -> &mut Self {
        f(self);
        self
    }
}

#[cfg(test)]
mod test {
    use glam::dvec3;

    use crate::{components::vpoint::VPointComponentVec, prelude::Partial};

    #[test]
    fn test_get_partial() {
        let points = VPointComponentVec(
            vec![
                dvec3(0.0, 0.0, 0.0),
                dvec3(1.0, 1.0, 1.0),
                dvec3(2.0, 2.0, 2.0),
                dvec3(2.0, 2.0, 2.0),
                dvec3(3.0, 3.0, 3.0),
                dvec3(4.0, 4.0, 4.0),
                dvec3(5.0, 5.0, 5.0),
            ]
            .into(),
        );
        let partial = points.get_partial(0.0..1.0);
        assert_eq!(partial, points);

        let partial = points.get_partial(0.0..0.5);
        println!("{:?}", partial);
    }

    #[test]
    fn test_put_start_and_end_on() {
        let mut points = VPointComponentVec(
            vec![
                dvec3(0.0, 0.0, 0.0),
                dvec3(1.0, 0.0, 0.0),
                dvec3(2.0, 2.0, 0.0),
            ]
            .into(),
        );
        points.put_start_and_end_on(dvec3(0.0, 0.0, 0.0), dvec3(4.0, 4.0, 0.0));
        assert_eq!(
            points.0,
            vec![
                dvec3(0.0, 0.0, 0.0),
                dvec3(2.0, 0.0, 0.0),
                dvec3(4.0, 4.0, 0.0),
            ]
            .into()
        );
        points.put_start_and_end_on(dvec3(0.0, 0.0, 0.0), dvec3(-2.0, -2.0, 0.0));
        assert_eq!(
            points.0,
            vec![
                dvec3(0.0, 0.0, 0.0),
                dvec3(0.0, 1.0, 0.0),
                dvec3(-2.0, -2.0, 0.0),
            ]
            .into()
        );
    }
}
