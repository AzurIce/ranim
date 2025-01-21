use std::ops::{Deref, DerefMut};

use glam::Vec3;
use itertools::Itertools;

use super::ComponentData;

/// VPoints is used to represent a bunch of cubic bezier paths.
///
/// Every 4 elements in the inner vector is a cubic bezier path
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VPoint(Vec3);

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

impl VPoint {
    pub const ZERO: Self = Self(Vec3::ZERO);
    pub const NAN: Self = Self(Vec3::NAN);

    pub fn new(vec3: Vec3) -> Self {
        Self(vec3)
    }
}

impl ComponentData<VPoint> {
    pub fn get_closepath_flags(&self) -> Vec<bool> {
        let mut flags = vec![false; self.len()];

        let mut i = 0;
        for (j, (a, b)) in self.iter().tuple_windows().enumerate() {
            // Subpath ends at j + 1 with a = end_p, b = NAN
            // or ends at vec boundary with b = end_p
            if *b == VPoint::NAN || j == self.len() - 2 {
                let end_p = if *b == VPoint::NAN { a } else { b };
                if end_p == self.get(i).unwrap() {
                    flags[i..=j + 1].fill(true);
                }
                i = j + 2;
            }
        }

        flags
    }
}
