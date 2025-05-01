use std::cmp::Ordering;
use std::ops::{Deref, DerefMut};

use glam::DMat3;
use glam::DVec3;
use itertools::Itertools;
use ranim_macros::Interpolatable;

use crate::traits::Alignable;
use crate::traits::BoundingBox;
use crate::traits::PointsFunc;
use crate::traits::Position;
use crate::utils::bezier::{get_subpath_closed_flag, trim_quad_bezier};
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

impl Alignable for VPointComponentVec {
    fn is_aligned(&self, other: &Self) -> bool {
        self.len() == other.len()
    }
    fn align_with(&mut self, other: &mut Self) {
        if self.is_empty() {
            self.0 = vec![DVec3::ZERO; 3].into();
        }
        if self.len() > other.len() {
            other.align_with(self);
        }

        let into_closed_subpaths = |subpaths: Vec<Vec<DVec3>>| -> Vec<Vec<DVec3>> {
            subpaths
                .into_iter()
                .map(|sp| {
                    // should have no zero-length subpath
                    if !get_subpath_closed_flag(&sp).unwrap().1 {
                        let sp_len = sp.len();
                        let sp_iter = sp.into_iter();
                        sp_iter
                            .clone()
                            .take(sp_len - 1)
                            .chain(sp_iter.rev())
                            .collect::<Vec<_>>()
                    } else {
                        sp
                    }
                })
                .collect::<Vec<_>>()
        };
        let sps_self = into_closed_subpaths(self.get_subpaths());
        let sps_other = into_closed_subpaths(other.get_subpaths());

        let sps_to_points = |mut sps: Vec<Vec<DVec3>>| -> Vec<DVec3> {
            let sps_cnt = sps.len();
            let last_sp = sps.split_off(sps_cnt - 1);
            sps.into_iter()
                .flat_map(|sp| sp.into_iter())
                .chain(last_sp[0].iter().cloned())
                .collect()
        };

        let points_self = sps_to_points(sps_self);
        let points_other = sps_to_points(sps_other);

        let points_to_bez_tuples = |points: &Vec<DVec3>| -> Vec<[DVec3; 3]> {
            let it0 = points.iter().step_by(2).cloned();
            let it1 = points.iter().skip(1).step_by(2).cloned();
            let it2 = points.iter().skip(2).step_by(2).cloned();
            it0.zip(it1).zip(it2).map(|((a, b), c)| [a, b, c]).collect()
        };
        let align_points = |points: Vec<DVec3>, len: usize| -> Vec<DVec3> {
            let bez_tuples = points_to_bez_tuples(&points);

            let diff_len = (len - points.len()) / 2;
            // println!("{:?}", bez_tuples);
            let mut lens = bez_tuples
                .iter()
                .map(|[a, b, c]| {
                    if (a - b).length_squared() < f64::EPSILON {
                        0.0
                    } else {
                        (c - a).length()
                    }
                })
                .collect::<Vec<_>>();
            let mut ipc = vec![0; bez_tuples.len()];

            for _ in 0..diff_len {
                // println!("{:?}", lens);
                let idx = lens
                    .iter()
                    .position_max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                    .unwrap();
                ipc[idx] += 1;
                lens[idx] *= ipc[idx] as f64 / (ipc[idx] + 1) as f64;
            }
            // println!("BEZ: {:?}", bez_tuples);
            // println!("IPC: {:?}", ipc);
            let new_segs = bez_tuples
                .into_iter()
                .zip(ipc)
                .map(|(bez, ipc)| {
                    // curve cnt is ipc + 1, anchor cnt is ipc + 2
                    let alphas = (0..ipc + 2)
                        .map(|i| i as f64 / (ipc + 1) as f64)
                        .collect::<Vec<_>>();
                    let mut new_points = Vec::with_capacity((ipc + 1) * 2 + 1);
                    new_points.push(bez[0]);
                    // println!("###bez: {:?}, ipc: {}", bez, ipc);
                    alphas.iter().tuple_windows().for_each(|(a1, a2)| {
                        let partial = trim_quad_bezier(&bez, *a1, *a2);
                        // println!("{} {}: {:?}", a1, a2, partial);
                        new_points.extend(partial[1..].iter())
                    });
                    // println!("{:?}", new_points);
                    new_points
                })
                .collect::<Vec<_>>();
            let mut new_points = Vec::with_capacity(other.len());
            new_points.extend_from_slice(&new_segs[0]);
            for seg in new_segs.into_iter().skip(1) {
                new_points.extend(&seg[1..]);
            }
            new_points
        };

        if points_self.len() < points_other.len() {
            self.0 = align_points(points_self, points_other.len()).into();
            other.0 = points_other.into();
        } else {
            other.0 = align_points(points_other, points_self.len()).into();
            self.0 = points_self.into();
        }
    }
}

// fn extend_subpath_with_n(mut subpath: Vec<DVec3>, n: usize) -> Vec<DVec3> {
//     let beziers = subpath.iter().zip(other)
// }

impl VPointComponentVec {
    pub fn get_subpaths(&self) -> Vec<Vec<DVec3>> {
        let mut subpaths = Vec::new();

        let mut subpath = Vec::new();
        for mut chunk in self.iter().chunks(2).into_iter() {
            let (a, b) = (chunk.next(), chunk.next());
            match (a, b) {
                (Some(a), Some(b)) => {
                    subpath.push(*a);
                    subpath.push(*b);
                    if a == b {
                        subpaths.push(std::mem::take(&mut subpath));
                    }
                }
                (Some(a), None) => {
                    subpath.push(*a);
                    subpaths.push(std::mem::take(&mut subpath))
                }
                _ => unreachable!(),
            }
        }

        subpaths
    }
    pub fn get_seg(&self, idx: usize) -> Option<&[DVec3; 3]> {
        self.get(idx * 2..idx * 2 + 3)
            .and_then(|seg| seg.try_into().ok())
    }

    pub fn get_closepath_flags(&self) -> Vec<bool> {
        let len = self.len();
        let mut flags = vec![false; len];

        // println!("{:?}", self.0);
        let mut i = 0;
        while let Some((end_idx, is_closed)) = self.get(i..).and_then(get_subpath_closed_flag) {
            // println!("{i} {end_idx} {len}");
            let end_idx = i + end_idx + 2;
            flags[i..=end_idx.clamp(i, len - 1)].fill(is_closed);
            i = end_idx;
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
        let rotate_angle = cur_v.angle_between(v);
        let mut rotate_axis = cur_v.cross(v).normalize();
        if rotate_axis.length_squared() <= f64::EPSILON {
            rotate_axis = DVec3::Z;
        }
        self.rotate_by_anchor(rotate_angle, rotate_axis, Anchor::Point(cur_start));
        self.shift(start - cur_start);

        self
    }
}

impl VPointComponentVec {
    pub fn get_partial(&self, range: std::ops::Range<f64>) -> Self {
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

    use crate::{
        components::vpoint::VPointComponentVec,
        items::{
            Blueprint,
            vitem::{Circle, Square},
        },
        traits::Alignable,
    };

    #[test]
    fn test_align() {
        let mut circle = Circle(1.0).build();
        let mut square = Square(1.0).build();
        println!("{:?}", square.vpoints);
        square.align_with(&mut circle);
        println!("{:?}", square.vpoints);
    }

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
