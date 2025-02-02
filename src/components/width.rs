use crate::prelude::{Interpolatable, Partial};

use super::{ComponentData, PointWise};

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
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Self(self.0.lerp(&target.0, t))
    }
}

// impl Partial for ComponentData<Width> {
//     fn get_partial(&self, range: std::ops::Range<f32>) -> Self {
//         let start = range.start * (self.len() - 1) as f32;
//         let end = range.end * (self.len() - 1) as f32;

//         let full_start_anchor_idx = (start.floor() + 1.0) as usize;
//         let full_end_anchor_idx = (end.ceil() - 1.0) as usize;

//         if end - start < 1.0 {
//             let vstart = self.0[full_start_anchor_idx - 1]
//                 .lerp(&self.0[full_start_anchor_idx], start.fract());
//             let vend = self.0[full_start_anchor_idx - 1]
//                 .lerp(&self.0[full_start_anchor_idx], end.fract());
//             return vec![vstart, vend].into();
//         }

//         let mut partial = Vec::with_capacity(full_end_anchor_idx - full_start_anchor_idx + 1 + 2);

//         let start_fract = 1.0 - (full_start_anchor_idx as f32 - start);
//         let start_v =
//             self.0[full_start_anchor_idx - 1].lerp(&self.0[full_start_anchor_idx], start_fract);
//         partial.push(start_v);

//         if let Some(part) = self.0.get(full_start_anchor_idx..=full_end_anchor_idx) {
//             partial.extend_from_slice(part);
//         }

//         let end_fract = end - full_end_anchor_idx as f32;
//         let end_v = self.0[full_end_anchor_idx].lerp(&self.0[full_end_anchor_idx + 1], end_fract);
//         partial.push(end_v);

//         partial.into()
//     }
// }

// impl Deref for Width {
//     type Target = f32;
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl DerefMut for Width {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.0
//     }
// }
