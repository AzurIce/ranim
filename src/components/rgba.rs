use std::ops::{Deref, DerefMut};

use glam::Vec4;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Rgba(pub Vec4);

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