use std::ops::{Deref, DerefMut};

use bevy_color::{ColorToComponents, LinearRgba};
use glam::{vec4, Vec4};

use crate::prelude::{Interpolatable, Opacity};

use super::ComponentData;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Rgba(pub Vec4);

impl Opacity for Rgba {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.0.w = opacity;
        self
    }
}

impl Opacity for ComponentData<Rgba> {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.iter_mut().for_each(|rgba| {
            rgba.set_opacity(opacity);
        });
        self
    }
}

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

impl Interpolatable for Rgba {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Self(self.0.lerp(target.0, t))
    }
}
