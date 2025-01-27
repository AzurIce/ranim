use std::ops::{Deref, DerefMut};

use glam::Vec3;

use crate::prelude::Alignable;
use crate::prelude::Interpolatable;

use super::Transform3d;

use super::ComponentData;

/// VPoints is used to represent a bunch of cubic bezier paths.
///
/// Every 4 elements in the inner vector is a cubic bezier path
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


impl ComponentData<VPoint> {
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
