use std::ops::{Deref, DerefMut};

// use bevy_color::{ColorToComponents, LinearRgba};
use color::{AlphaColor, ColorSpace, LinearSrgb, Srgb};
use glam::{vec4, Vec4};

use crate::prelude::{Interpolatable, Opacity};

use super::{ComponentVec, PointWise};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Rgba(pub Vec4);

impl PointWise for Rgba {}

impl Opacity for Rgba {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.0.w = opacity;
        self
    }
}

impl Opacity for ComponentVec<Rgba> {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.iter_mut().for_each(|rgba| {
            rgba.set_opacity(opacity);
        });
        self
    }
}

impl<CS: ColorSpace> From<AlphaColor<CS>> for Rgba {
    fn from(value: AlphaColor<CS>) -> Self {
        let rgba = value.convert::<LinearSrgb>().components;
        Self(Vec4::from_array(rgba))
    }
}

impl From<Rgba> for AlphaColor<Srgb> {
    fn from(value: Rgba) -> AlphaColor<Srgb> {
        let linear_rgba = value.0.to_array();
        AlphaColor::<LinearSrgb>::new(linear_rgba).convert()
    }
}

// impl From<bevy_color::Srgba> for Rgba {
//     fn from(value: bevy_color::Srgba) -> Self {
//         let rgba = LinearRgba::from(value);
//         Self(rgba.to_vec4())
//     }
// }

impl Default for Rgba {
    fn default() -> Self {
        vec4(1.0, 0.0, 0.0, 1.0).into()
    }
}

impl From<Vec4> for Rgba {
    fn from(value: Vec4) -> Self {
        Self(value)
    }
}

impl Deref for Rgba {
    type Target = Vec4;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Rgba {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Interpolatable for Rgba {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Self(self.0.lerp(target.0, t))
    }
}

// impl Partial for ComponentData<Rgba> {
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

//         let mut partial =
//             Vec::with_capacity(full_end_anchor_idx - full_start_anchor_idx + 1 + 2);

//         let start_fract = 1.0 - (full_start_anchor_idx as f32 - start);
//         let start_v = self.0[full_start_anchor_idx - 1]
//             .lerp(&self.0[full_start_anchor_idx], start_fract);
//         partial.push(start_v);

//         if let Some(part) = self
//             .0
//             .get(full_start_anchor_idx..=full_end_anchor_idx)
//         {
//             partial.extend_from_slice(part);
//         }

//         let end_fract = end - full_end_anchor_idx as f32;
//         let end_v = self.0[full_end_anchor_idx]
//             .lerp(&self.0[full_end_anchor_idx + 1], end_fract);
//         partial.push(end_v);

//         partial.into()
//     }
// }

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_convertion() {
        let approx = |a: f32, b: f32| (a - b).abs() < 0.001;
        // The `rgb8` and `rgba8` should be in srgb
        let color = AlphaColor::from_rgb8(85, 133, 217);
        assert!(approx(color.components[0], 0.333));
        assert!(approx(color.components[1], 0.522));
        assert!(approx(color.components[2], 0.851));

        let linear_rgba = Rgba::from(color);
        assert!(approx(linear_rgba.x, 0.091));
        assert!(approx(linear_rgba.y, 0.235));
        assert!(approx(linear_rgba.z, 0.694));
    }
}
