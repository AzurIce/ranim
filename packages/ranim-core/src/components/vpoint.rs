use std::cmp::Ordering;

use derive_more::{Deref, DerefMut};
use glam::DVec3;
use itertools::Itertools;

use crate::anchor::Aabb;
use crate::traits::*;
use crate::utils::bezier::{get_subpath_closed_flag, trim_quad_bezier};
use crate::utils::math::interpolate_usize;
use crate::utils::{avg, resize_preserving_order_with_repeated_indices};

fn points_aabb(points: &mut impl Iterator<Item = DVec3>) -> [DVec3; 2] {
    if let Some(first) = points.next() {
        let (mut min, mut max) = (first, first);
        for point in points {
            min = min.min(point);
            max = max.max(point);
        }
        [min, max]
    } else {
        [DVec3::ZERO; 2]
    }
}

fn bezier_aabb(p1: DVec3, p2: DVec3, p3: DVec3) -> [DVec3; 2] {
    let t_extremum = (p1 - p2) / (p1 - 2. * p2 + p3);
    points_aabb(
        &mut <[f64; 3]>::from(t_extremum)
            .into_iter()
            .filter(|&t| (0. ..=1.).contains(&t))
            .map(|t| (1. - t).powi(2) * p1 + 2. * t * (1. - t) * p2 + t.powi(2) * p3)
            .chain([p1, p3]),
    )
}

/// A Vec of VPoint Data. It is used to represent a bunch of quad bezier paths.
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
#[derive(Debug, Clone, PartialEq, Deref, DerefMut, ranim_macros::Interpolatable)]
pub struct VPointVec(pub Vec<DVec3>);

impl Aabb for VPointVec {
    fn aabb(&self) -> [DVec3; 2] {
        let mut iter = self.0.iter().cloned();
        if let Some(first) = iter.next() {
            let (mut min, mut max) = (first, first);
            let mut p1 = first;
            loop {
                if let Some(p2) = iter.next() {
                    if let Some(p3) = iter.next() {
                        let [bezier_min, bezier_max] = bezier_aabb(p1, p2, p3);
                        min = min.min(bezier_min);
                        max = max.max(bezier_max);
                        p1 = p3;
                    } else {
                        unreachable!()
                    }
                } else {
                    return [min, max];
                }
            }
        } else {
            [DVec3::ZERO; 2]
        }
    }
}

impl AsRef<[DVec3]> for VPointVec {
    fn as_ref(&self) -> &[DVec3] {
        self.0.as_ref()
    }
}

impl AsMut<[DVec3]> for VPointVec {
    fn as_mut(&mut self) -> &mut [DVec3] {
        self.0.as_mut()
    }
}

impl Shift for VPointVec {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.as_mut().shift(offset);
        self
    }
}

impl Rotate for VPointVec {
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self {
        self.as_mut().rotate_at_point(angle, axis, point);
        self
    }
}

impl Scale for VPointVec {
    fn scale_at_point(&mut self, scale: DVec3, point: DVec3) -> &mut Self {
        self.as_mut().scale_at_point(scale, point);
        self
    }
}

impl Alignable for VPointVec {
    fn is_aligned(&self, other: &Self) -> bool {
        self.len() == other.len()
    }
    fn align_with(&mut self, other: &mut Self) {
        if self.is_empty() {
            self.0 = vec![DVec3::ZERO; 3];
        }
        if self.len() > other.len() {
            other.align_with(self);
            return;
        }

        let into_closed_subpaths = |subpaths: Vec<Vec<DVec3>>| -> Vec<Vec<DVec3>> {
            subpaths
                .into_iter()
                .map(|sp| {
                    // should have no zero-length subpath
                    if !get_subpath_closed_flag(&sp).map(|f| f.1).unwrap() {
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
        let mut sps_self = into_closed_subpaths(self.get_subpaths());
        let mut sps_other = into_closed_subpaths(other.get_subpaths());
        let len = sps_self.len().max(sps_other.len());
        if sps_self.len() != len {
            let (mut x, idxs) = resize_preserving_order_with_repeated_indices(&sps_self, len);
            for idx in idxs {
                let center = avg(&x[idx]);
                x[idx].fill(center);
            }
            sps_self = x;
        }
        if sps_other.len() != len {
            let (mut x, idxs) = resize_preserving_order_with_repeated_indices(&sps_other, len);
            for idx in idxs {
                let center = avg(&x[idx]);
                x[idx].fill(center);
            }
            sps_other = x;
        }

        let points_to_bez_tuples = |points: &Vec<DVec3>| -> Vec<[DVec3; 3]> {
            let it0 = points.iter().step_by(2).cloned();
            let it1 = points.iter().skip(1).step_by(2).cloned();
            let it2 = points.iter().skip(2).step_by(2).cloned();
            it0.zip(it1).zip(it2).map(|((a, b), c)| [a, b, c]).collect()
        };
        let align_points = |points: &Vec<DVec3>, len: usize| -> Vec<DVec3> {
            let bez_tuples = points_to_bez_tuples(points);

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
            let mut ipc = vec![0usize; bez_tuples.len()];

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
                        .map(|i: usize| i as f64 / (ipc + 1) as f64)
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

        sps_self
            .iter_mut()
            .zip(sps_other.iter_mut())
            .for_each(|(sp_a, sp_b)| {
                // println!("sp align: {} {}", sp_a.len(), sp_b.len());
                let len = sp_a.len().max(sp_b.len());
                if sp_a.len() != len {
                    *sp_a = align_points(sp_a, len)
                }
                if sp_b.len() != len {
                    *sp_b = align_points(sp_b, len)
                }
            });

        let sps_to_points = |sps: Vec<Vec<DVec3>>| -> Vec<DVec3> {
            let mut points = sps
                .into_iter()
                .flat_map(|sp| {
                    let last = *sp.last().unwrap();
                    sp.into_iter().chain(std::iter::once(last))
                })
                .collect::<Vec<_>>();
            points.pop();
            points
        };

        self.0 = sps_to_points(sps_self);
        other.0 = sps_to_points(sps_other);
    }
}

// fn extend_subpath_with_n(mut subpath: Vec<DVec3>, n: usize) -> Vec<DVec3> {
//     let beziers = subpath.iter().zip(other)
// }

impl VPointVec {
    /// Get Subpaths
    pub fn get_subpaths(&self) -> Vec<Vec<DVec3>> {
        let mut subpaths = Vec::new();

        let mut subpath = Vec::new();
        let mut iter_a = self.iter().step_by(2).peekable();
        let mut iter_b = self.iter().skip(1).step_by(2).peekable();

        loop {
            match (iter_a.next(), iter_b.next()) {
                (Some(a), Some(b)) => {
                    subpath.push(*a);
                    if a != b {
                        subpath.push(*b);
                    } else {
                        while let (Some(c), Some(d)) = (iter_a.peek(), iter_b.peek())
                            && b == *c
                            && c == d
                        {
                            subpath.extend([**c; 2]);
                            iter_a.next();
                            iter_b.next();
                        }
                        assert!(subpath.len() % 2 != 0);
                        subpaths.push(std::mem::take(&mut subpath));
                    }
                }
                (Some(a), None) => {
                    subpath.push(*a);
                    assert!(subpath.len() % 2 != 0);
                    subpaths.push(std::mem::take(&mut subpath));
                    break;
                }
                _ => unreachable!(),
            }
        }

        // for sp in &subpaths {
        //     println!("{}\n - {:?}", sp.len(), sp);
        // }

        subpaths
    }
    /// Get the segment
    pub fn get_seg(&self, idx: usize) -> Option<&[DVec3; 3]> {
        self.get(idx * 2..idx * 2 + 3)
            .and_then(|seg| seg.try_into().ok())
    }
    /// Get closed path flags
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
        self.scale_at(DVec3::splat(v.length() / cur_v.length()), cur_start);
        let rotate_angle = cur_v.angle_between(v);
        let mut rotate_axis = cur_v.cross(v);
        if rotate_axis.length_squared() <= f64::EPSILON {
            rotate_axis = DVec3::Z;
        }
        rotate_axis = rotate_axis.normalize();
        self.rotate_at(rotate_angle, rotate_axis, cur_start);
        self.shift(start - cur_start);

        self
    }

    /// Get partial of the vpoint.
    ///
    /// This will trim the bezier.
    pub fn get_partial(&self, range: std::ops::Range<f64>) -> Self {
        let max_anchor_idx = self.len() / 2;

        let (start_index, start_residue) = interpolate_usize(0, max_anchor_idx, range.start);
        let (end_index, end_residue) = interpolate_usize(0, max_anchor_idx, range.end);

        if end_index - start_index == 0 {
            let seg = self.get_seg(start_index).unwrap().map(|p| p);
            let quad = trim_quad_bezier(&seg, start_residue, end_residue);
            VPointVec(quad.into())
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

            VPointVec(partial)
        }
    }
}

#[cfg(test)]
mod test {
    use std::f64::consts::PI;

    use assert_float_eq::assert_float_absolute_eq;
    use glam::{DVec3, dvec3};

    use crate::{
        components::vpoint::VPointVec,
        traits::{Aabb as _, RotateExt},
    };

    #[test]
    fn test_get_subpath() {
        let points = VPointVec(vec![DVec3::ZERO; 9]);
        let sps = points.get_subpaths();
        println!("{:?}", sps);
        let points = VPointVec(vec![
            DVec3::X,
            DVec3::Y,
            DVec3::Z,
            DVec3::Z,
            DVec3::NEG_X,
            DVec3::NEG_Y,
            DVec3::ZERO,
        ]);
        let sps = points.get_subpaths();
        println!("{:?}", sps);
        let points = VPointVec(vec![DVec3::X, DVec3::Y, DVec3::Z]);
        let sps = points.get_subpaths();
        println!("{:?}", sps);
        let points = VPointVec(vec![DVec3::X, DVec3::Y, DVec3::Z, DVec3::Z, DVec3::Z]);
        let sps = points.get_subpaths();
        println!("{:?}", sps);
    }

    #[test]
    fn test_get_partial() {
        let points = VPointVec(vec![
            dvec3(0.0, 0.0, 0.0),
            dvec3(1.0, 1.0, 1.0),
            dvec3(2.0, 2.0, 2.0),
            dvec3(2.0, 2.0, 2.0),
            dvec3(3.0, 3.0, 3.0),
            dvec3(4.0, 4.0, 4.0),
            dvec3(5.0, 5.0, 5.0),
        ]);
        let partial = points.get_partial(0.0..1.0);
        assert_eq!(partial, points);

        let partial = points.get_partial(0.0..0.5);
        println!("{partial:?}");
    }

    #[test]
    fn test_rotate() {
        let mut points = VPointVec(vec![
            dvec3(0.0, 0.0, 0.0),
            dvec3(1.0, 0.0, 0.0),
            dvec3(2.0, 2.0, 0.0),
        ]);
        points.rotate_at(PI, DVec3::Z, DVec3::ZERO);
        points
            .0
            .iter()
            .zip([
                dvec3(0.0, 0.0, 0.0),
                dvec3(-1.0, 0.0, 0.0),
                dvec3(-2.0, -2.0, 0.0),
            ])
            .for_each(|(res, truth)| {
                assert_float_absolute_eq!(res.distance_squared(truth), 0.0, 1e-10);
            });
    }

    #[test]
    fn test_put_start_and_end_on() {
        let mut points = VPointVec(vec![
            dvec3(0.0, 0.0, 0.0),
            dvec3(1.0, 0.0, 0.0),
            dvec3(2.0, 2.0, 0.0),
        ]);
        points.put_start_and_end_on(dvec3(0.0, 0.0, 0.0), dvec3(4.0, 4.0, 0.0));
        points
            .0
            .iter()
            .zip([
                dvec3(0.0, 0.0, 0.0),
                dvec3(2.0, 0.0, 0.0),
                dvec3(4.0, 4.0, 0.0),
            ])
            .for_each(|(res, truth)| {
                assert_float_absolute_eq!(res.distance_squared(truth), 0.0, 1e-10);
            });

        points.put_start_and_end_on(dvec3(0.0, 0.0, 0.0), dvec3(-2.0, -2.0, 0.0));
        points
            .0
            .iter()
            .zip([
                dvec3(0.0, 0.0, 0.0),
                dvec3(-1.0, 0.0, 0.0),
                dvec3(-2.0, -2.0, 0.0),
            ])
            .for_each(|(res, truth)| {
                assert_float_absolute_eq!(res.distance_squared(truth), 0.0, 1e-10);
            });
    }

    #[test]
    fn test_aabb() {
        let points = VPointVec(vec![
            dvec3(-2., 1., 0.),
            dvec3(0., -1., 0.),
            dvec3(2., 1., 0.),
        ]); // parabola y = x^2 / 4 with x in [-2, 2]
        let [min, max] = points.aabb();
        assert_float_absolute_eq!(min.distance(dvec3(-2., 0., 0.)), 0.0, 1e-10);
        assert_float_absolute_eq!(max.distance(dvec3(2., 1., 0.)), 0.0, 1e-10);
    }
}
