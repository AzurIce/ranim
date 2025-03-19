use color::{AlphaColor, ColorSpace};
use glam::Mat4;

pub trait Interpolatable {
    fn lerp(&self, target: &Self, t: f32) -> Self;
}

impl Interpolatable for f32 {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        self + (target - self) * t
    }
}

impl<CS: ColorSpace> Interpolatable for AlphaColor<CS> {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        // TODO: figure out to use `lerp_rect` or `lerp`
        AlphaColor::lerp_rect(*self, *other, t)
    }
}

impl Interpolatable for Mat4 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let mut result = Mat4::ZERO;
        for i in 0..4 {
            for j in 0..4 {
                result.col_mut(i)[j] = self.col(i)[j].lerp(&other.col(i)[j], t);
            }
        }
        result
    }
}
