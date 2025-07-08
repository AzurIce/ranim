use derive_more::From;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::prelude::Interpolatable;

use super::PointWise;

/// Width
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, bytemuck::Pod, bytemuck::Zeroable, From)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Width(pub f32);

impl PointWise for Width {}

impl Width {
    /// Max
    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }
    /// Min
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }
}

impl Default for Width {
    fn default() -> Self {
        1.0.into()
    }
}

impl Interpolatable for Width {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self(self.0.lerp(&target.0, t))
    }
}
