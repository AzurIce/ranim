use crate::prelude::Interpolatable;

use super::PointWise;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Width(pub f32);

impl PointWise for Width {}

impl Width {
    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }
}

impl Default for Width {
    fn default() -> Self {
        1.0.into()
    }
}

impl From<f32> for Width {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl Interpolatable for Width {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self(self.0.lerp(&target.0, t))
    }
}
