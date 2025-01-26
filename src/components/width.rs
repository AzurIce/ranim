#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Width(pub f32);

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
