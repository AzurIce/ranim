use std::cmp::Ordering;

use derive_more::{Deref, DerefMut};
use glam::{DVec2, DVec3};
use itertools::Itertools;

use crate::traits::*;
use crate::utils::bezier::{get_subpath_closed_flag, trim_quad_bezier};
use crate::utils::math::interpolate_usize;
use crate::utils::{avg, resize_preserving_order_with_repeated_indices};

use super::ComponentVec;

/// A point in bezier path.
///
/// `x`, `y`

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
#[derive(Debug, Clone, PartialEq, Deref, DerefMut)]
pub struct VPointComponentVec(pub ComponentVec<DVec3>);

impl Interpolatable for VPointComponentVec {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self(self.0.lerp(&target.0, t))
    }
}

impl Alignable for VPointComponentVec {
    fn is_aligned(&self, other: &Self) -> bool {
        self.len() == other.len()
    }
    fn align_with(&mut self, other: &mut Self) {
        // println!(
        //     "VPointComponentVec::align_with: {} {}",
        //     self.len(),
        //     other.len()
        // );
        if self.is_empty() {
            self.0 = vec![DVec3::ZERO; 3].into();
        }
        if self.len() > other.len() {
            other.align_with(self);
            return;
        }

        let into_closed_subpaths = |subpaths: Vec<Vec<DVec3>>| -> Vec<Vec<DVec3>> {
            subpaths
                .into_iter()
                .map(|sp| {
                    // println!("into_closed_subpaths {}", sp.len());
                    // if sp.len() == 1 {
                    //     return vec![sp[0]; 3];
                    // }
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
        // println!("self: {}", sps_self.len());
        // for (i, sp) in sps_self.iter().enumerate() {
        //     println!("[{i}] {} {:?}", sp.len(), sp);
        // }
        // println!("other: {}", sps_other.len());
        // for (i, sp) in sps_other.iter().enumerate() {
        //     println!("[{i}] {} {:?}", sp.len(), sp);
        // }
        let len = sps_self.len().max(sps_other.len());
        // println!("#####{len}#####");
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
        // println!("self: {}", sps_self.len());
        // for (i, sp) in sps_self.iter().enumerate() {
        //     println!("[{i}] {} {:?}", sp.len(), sp);
        // }
        // println!("other: {}", sps_other.len());
        // for (i, sp) in sps_other.iter().enumerate() {
        //     println!("[{i}] {} {:?}", sp.len(), sp);
        // }

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
        // println!("self: {}", sps_self.len());
        // for (i, sp) in sps_self.iter().enumerate() {
        //     println!("[{i}] {} {:?}", sp.len(), sp);
        // }
        // println!("other: {}", sps_other.len());
        // for (i, sp) in sps_other.iter().enumerate() {
        //     println!("[{i}] {} {:?}", sp.len(), sp);
        // }

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

        let points_self = sps_to_points(sps_self);
        let points_other = sps_to_points(sps_other);

        self.0 = points_self.into();
        other.0 = points_other.into();
    }
}

// fn extend_subpath_with_n(mut subpath: Vec<DVec3>, n: usize) -> Vec<DVec3> {
//     let beziers = subpath.iter().zip(other)
// }

impl VPointComponentVec {
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
        self.scale_by_anchor(
            DVec3::splat(v.length() / cur_v.length()),
            Anchor::Point(cur_start),
        );
        let rotate_angle = cur_v.angle_between(v);
        let mut rotate_axis = cur_v.cross(v);
        if rotate_axis.length_squared() <= f64::EPSILON {
            rotate_axis = DVec3::Z;
        }
        rotate_axis = rotate_axis.normalize();
        self.rotate_by_anchor(rotate_angle, rotate_axis, Anchor::Point(cur_start));
        self.shift(start - cur_start);

        self
    }
}

impl VPointComponentVec {
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

impl PointsFunc for [DVec3] {
    fn apply_points_func(&mut self, f: impl for<'a> Fn(&'a mut [DVec3])) -> &mut Self {
        f(self);
        self
    }
}

#[cfg(test)]
mod test {
    use std::f64::consts::PI;

    use assert_float_eq::assert_float_absolute_eq;
    use glam::{DVec3, dvec3};

    use crate::{
        components::{ComponentVec, vpoint::VPointComponentVec},
        traits::{Anchor, Rotate},
    };

    #[test]
    fn test_get_subpath() {
        let points = VPointComponentVec(ComponentVec(vec![DVec3::ZERO; 9]));
        let sps = points.get_subpaths();
        println!("{:?}", sps);
        let points = VPointComponentVec(ComponentVec(vec![
            DVec3::X,
            DVec3::Y,
            DVec3::Z,
            DVec3::Z,
            DVec3::NEG_X,
            DVec3::NEG_Y,
            DVec3::ZERO,
        ]));
        let sps = points.get_subpaths();
        println!("{:?}", sps);
        let points = VPointComponentVec(ComponentVec(vec![DVec3::X, DVec3::Y, DVec3::Z]));
        let sps = points.get_subpaths();
        println!("{:?}", sps);
        let points = VPointComponentVec(ComponentVec(vec![
            DVec3::X,
            DVec3::Y,
            DVec3::Z,
            DVec3::Z,
            DVec3::Z,
        ]));
        let sps = points.get_subpaths();
        println!("{:?}", sps);
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
        println!("{partial:?}");
    }

    #[test]
    fn test_rotate() {
        let mut points = VPointComponentVec(
            vec![
                dvec3(0.0, 0.0, 0.0),
                dvec3(1.0, 0.0, 0.0),
                dvec3(2.0, 2.0, 0.0),
            ]
            .into(),
        );
        points.rotate_by_anchor(PI, DVec3::Z, Anchor::Point(DVec3::ZERO));
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
        let mut points = VPointComponentVec(
            vec![
                dvec3(0.0, 0.0, 0.0),
                dvec3(1.0, 0.0, 0.0),
                dvec3(2.0, 2.0, 0.0),
            ]
            .into(),
        );
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
}
