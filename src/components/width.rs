use std::ops::{Deref, DerefMut};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Width(pub f32);

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

impl Deref for Width {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Width {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
