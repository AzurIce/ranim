use bevy_color::{LinearRgba, Srgba};

pub trait Interpolatable {
    fn lerp(&self, target: &Self, t: f32) -> Self;
}

impl Interpolatable for f32 {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        self + (target - self) * t
    }
}

impl Interpolatable for LinearRgba {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Self {
            red: self.red.lerp(&target.red, t),
            green: self.green.lerp(&target.green, t),
            blue: self.blue.lerp(&target.blue, t),
            alpha: self.alpha.lerp(&target.alpha, t),
        }
    }
}

impl Interpolatable for Srgba {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            red: self.red.lerp(&other.red, t),
            green: self.green.lerp(&other.green, t),
            blue: self.blue.lerp(&other.blue, t),
            alpha: self.alpha.lerp(&other.alpha, t),
        }
    }
}