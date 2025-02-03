use std::ops::{Deref, DerefMut};

use glam::Vec3;

use crate::prelude::Interpolatable;
use crate::prelude::Partial;
use crate::utils::bezier::trim_quad_bezier;
use crate::utils::math::interpolate_usize;

use super::Transform3d;

use super::ComponentData;

/// VPoints is used to represent a bunch of quad bezier paths.
///
/// Every 3 elements in the inner vector is a quad bezier path
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VPoint(pub Vec3);

impl Default for VPoint {
    fn default() -> Self {
        Vec3::ZERO.into()
    }
}

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

impl Transform3d for VPoint {
    fn position(&self) -> Vec3 {
        self.0
    }
    fn position_mut(&mut self) -> &mut Vec3 {
        &mut self.0
    }
}

impl Interpolatable for VPoint {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Self(self.0.lerp(target.0, t))
    }
}

impl Partial for ComponentData<VPoint> {
    fn get_partial(&self, range: std::ops::Range<f32>) -> Self {
        let max_anchor_idx = self.len() / 2;

        let (start_index, start_residue) = interpolate_usize(0, max_anchor_idx, range.start);
        let (end_index, end_residue) = interpolate_usize(0, max_anchor_idx, range.end);
        if start_index == end_index {
            let seg = self.get_seg(start_index).unwrap().map(|p| p.0);
            let quad = trim_quad_bezier(&seg, start_residue, end_residue);
            quad.into()
        } else {
            let seg = self.get_seg(start_index).unwrap().map(|p| p.0);
            let start_part = trim_quad_bezier(&seg, start_residue, 1.0);
            let seg = self.get_seg(end_index).unwrap().map(|p| p.0);
            let end_part = trim_quad_bezier(&seg, 0.0, end_residue);
            let mut partial = Vec::with_capacity((end_index - start_index + 1 + 2) * 2 + 1);
            partial.extend_from_slice(&start_part);
            if start_index + 1 <= end_index - 1 {
                partial.extend(
                    self.get((start_index + 1) * 2 + 1..(end_index - 1) * 2 + 2)
                        .unwrap()
                        .iter()
                        .map(|p| p.0),
                );
            }
            partial.extend_from_slice(&end_part);
            // trace!("vpoint: {:?}", partial.len());
            partial.into()
        }
    }
}

impl ComponentData<VPoint> {
    pub fn get_seg(&self, idx: usize) -> Option<&[VPoint; 3]> {
        self.get(idx * 2..idx * 2 + 3)
            .map(|seg| seg.try_into().ok())
            .flatten()
    }
    pub fn get_closepath_flags(&self) -> Vec<bool> {
        let mut flags = vec![false; self.len()];

        // println!("{:?}", self.0);
        let mut i = 0;
        for (j, chunk) in self.0[1..].chunks_exact(2).enumerate() {
            // println!("{:?}", (j, chunk));
            let cur_idx = 1 + j * 2;
            let [a, b] = chunk else { unreachable!() };
            if b == a || cur_idx == self.len() - 2 {
                if b == self.get(i).unwrap() {
                    flags[i..=cur_idx + 1].fill(true);
                }
                i = cur_idx + 2;
            }
        }

        // println!("{:?}", flags);

        flags
    }
}
