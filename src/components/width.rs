use std::ops::{Deref, DerefMut};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Width(pub f32);

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
