use std::ops::{Deref, DerefMut};

use bevy_color::{ColorToComponents, LinearRgba};
use glam::{vec4, Vec4};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Rgba(pub Vec4);

impl From<bevy_color::Srgba> for Rgba {
    fn from(value: bevy_color::Srgba) -> Self {
        let rgba = LinearRgba::from(value);
        Self(rgba.to_vec4())
    }
}

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
