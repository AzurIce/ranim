use std::ops::{Deref, DerefMut};

use glam::Vec3;
use itertools::Itertools;

use crate::prelude::Interpolatable;
use crate::prelude::Partial;
use crate::utils::bezier::trim_quad_bezier;
use crate::utils::math::interpolate_usize;

use super::ComponentVec;

/// VPoints is used to represent a bunch of quad bezier paths.
///
/// Every 3 elements in the inner vector is a quad bezier path
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct VPoint(pub Vec3);

impl From<Vec3> for VPoint {
    fn from(value: Vec3) -> Self {
        Self(value)
    }
}

impl Deref for VPoint {
    type Target = Vec3;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for VPoint {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Interpolatable for VPoint {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Self(self.0.lerp(target.0, t))
    }
}

impl Partial for ComponentVec<VPoint> {
    fn get_partial(&self, range: std::ops::Range<f32>) -> Self {
        // trace!("get_partial: {:?}", range);
        let max_anchor_idx = self.len() / 2;
        // trace!("max_anchor_idx: {}", max_anchor_idx);

        let (start_index, start_residue) = interpolate_usize(0, max_anchor_idx, range.start);
        let (end_index, end_residue) = interpolate_usize(0, max_anchor_idx, range.end);
        // println!("{} {}", self.len(), max_anchor_idx);
        // println!(
        //     "{:?}, start: {} {}, end: {} {}",
        //     range, start_index, start_residue, end_index, end_residue
        // );
        if end_index - start_index == 0 {
            let seg = self.get_seg(start_index).unwrap().map(|p| p.0);
            let quad = trim_quad_bezier(&seg, start_residue, end_residue);
            quad.into()
        } else {
            let mut partial = Vec::with_capacity((end_index - start_index + 1 + 2) * 2 + 1);

            let seg = self.get_seg(start_index).unwrap().map(|p| p.0);
            let start_part = trim_quad_bezier(&seg, start_residue, 1.0);
            partial.extend_from_slice(&start_part);
            // println!("start_seg: {:?}, start_part: {:?}", seg, start_part);

            // If start_index < end_index - 1, we need to add the middle segment
            //  start     mid    end
            // [o - o] [- o - o] [- o]
            if end_index - start_index > 1 {
                let mid = self
                    .get((start_index + 1) * 2 + 1..=end_index * 2)
                    .unwrap()
                    .iter()
                    .map(|p| p.0);
                // trace!("mid: {}", mid.len());
                partial.extend(mid);
            }

            if end_residue != 0.0 {
                let seg = self.get_seg(end_index).unwrap().map(|p| p.0);
                let end_part = trim_quad_bezier(&seg, 0.0, end_residue);
                // println!("end_seg: {:?}, end_part: {:?}", seg, end_part);
                partial.extend_from_slice(&end_part[1..]);
            }

            partial.into()
        }
    }
}

impl ComponentVec<VPoint> {
    pub fn get_seg(&self, idx: usize) -> Option<&[VPoint; 3]> {
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
                    // println!("chunk[{ia}, {ib}] {:?}", [a, b]);
                    if a == b {
                        Some((ia, a))
                    } else {
                        None
                    }
                }
                (Some((ia, a)), None) => Some((ia, a)),
                _ => unreachable!(),
            } {
                // println!("### path end ###");
                if (a.0 - self.get(i).unwrap().0).length_squared() <= 0.0001 {
                    // println!("### path closed ###");
                    flags[i..=ia].fill(true);
                }
                i = ia + 2;
            }
        }

        // println!("{:?}", flags);

        flags
    }
}

#[cfg(test)]
mod test {
    use glam::vec3;

    use crate::{components::ComponentVec, prelude::Partial};

    use super::VPoint;

    #[test]
    fn test_get_partial() {
        let points: ComponentVec<VPoint> = vec![
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 1.0, 1.0),
            vec3(2.0, 2.0, 2.0),
            vec3(2.0, 2.0, 2.0),
            vec3(3.0, 3.0, 3.0),
            vec3(4.0, 4.0, 4.0),
            vec3(5.0, 5.0, 5.0),
        ]
        .into();
        let partial = points.get_partial(0.0..1.0);
        assert_eq!(partial, points);

        let partial = points.get_partial(0.0..0.5);
        println!("{:?}", partial);
    }
}
