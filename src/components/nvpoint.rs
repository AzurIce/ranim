use std::ops::{Deref, DerefMut};

use glam::Vec3;

use super::{ComponentVec, PointWise, Transform3dComponent, Transformable};
use crate::traits::Interpolatable;

/// VPoints is used to represent a bunch of quad bezier paths.
///
/// Every 3 elements in the inner vector is a quad bezier path
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct NVPoint(pub [Vec3; 3]);

impl PointWise for NVPoint {}

impl From<[Vec3; 3]> for NVPoint {
    fn from(value: [Vec3; 3]) -> Self {
        Self(value)
    }
}

impl Interpolatable for NVPoint {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Self([
            self.0[0].lerp(target.0[0], t),
            self.0[1].lerp(target.0[1], t),
            self.0[2].lerp(target.0[2], t),
        ])
    }
}

pub trait NVPointSliceMethods {
    fn get_seg(&self, idx: usize) -> Option<&[NVPoint; 3]>;
    fn get_closepath_flags(&self) -> Vec<bool>;
}

impl NVPointSliceMethods for [NVPoint] {
    fn get_seg(&self, idx: usize) -> Option<&[NVPoint; 3]> {
        self.get(idx * 2..idx * 2 + 3)
            .and_then(|seg| seg.try_into().ok())
    }
    fn get_closepath_flags(&self) -> Vec<bool> {
        let len = self.len();
        let mut flags = vec![false; len];

        // println!("{:?}", self);
        // println!("start: {:?}", self.get(0).unwrap());
        let mut i = 0;
        for (idx, NVPoint([_h_prev, p, h_next])) in self.iter().enumerate() {
            // println!("[{}] h_prev: {:?}, p: {:?}, h_next: {:?}", idx, h_prev, p, h_next);
            if p == h_next || idx == len - 1 {
                // println!("### path end ###");
                if (p - self.get(i).unwrap().0[1]).length_squared() <= 0.0001 {
                    // println!("### path closed ###");
                    flags[i..=idx].fill(true);
                }
                i = idx + 1;
                // println!("start: {:?}", self.get(i));
            }
        }

        // println!("{:?}", flags);

        flags
    }
}

impl Transform3dComponent for NVPoint {
    fn pos(&self) -> Vec3 {
        self.0[1]
    }
    
    fn iter_points(&self) -> impl Iterator<Item = &Vec3> {
        self.0.iter()
    }

    fn iter_points_mut(&mut self) -> impl Iterator<Item = &mut Vec3> {
        self.0.iter_mut()
    }
}

#[cfg(test)]
mod test {
    use glam::vec3;

    use super::*;

    #[test]
    fn test_get_closepath_flags() {
        let points = vec![
            NVPoint([
                vec3(0.0, 0.0, 0.0),
                vec3(1.0, 1.0, 1.0),
                vec3(2.0, 2.0, 2.0),
            ]),
            NVPoint([
                vec3(2.0, 2.0, 2.0),
                vec3(3.0, 3.0, 3.0),
                vec3(4.0, 4.0, 4.0),
            ]),
        ];
        let flags = points.get_closepath_flags();
        assert_eq!(flags, vec![false; 2]);

        let points = vec![
            NVPoint([
                vec3(0.0, 0.0, 0.0),
                vec3(1.0, 1.0, 1.0),
                vec3(2.0, 2.0, 2.0),
            ]),
            NVPoint([
                vec3(2.0, 2.0, 2.0),
                vec3(3.0, 3.0, 3.0),
                vec3(4.0, 4.0, 4.0),
            ]),
            NVPoint([
                vec3(4.0, 4.0, 4.0),
                vec3(5.0, 5.0, 5.0),
                vec3(6.0, 6.0, 6.0),
            ]),
            NVPoint([
                vec3(6.0, 6.0, 6.0),
                vec3(1.0, 1.0, 1.0),
                vec3(0.0, 0.0, 0.0),
            ]),
        ];
        let flags = points.get_closepath_flags();
        assert_eq!(flags, vec![true; 4]);
    }
}
