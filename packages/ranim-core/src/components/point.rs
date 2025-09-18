use derive_more::{AsMut, AsRef, Deref, DerefMut, From};
use glam::DVec3;

use crate::prelude::Interpolatable;

/// Point
#[derive(Default, Debug, Clone, Copy, PartialEq, Deref, DerefMut, From, AsRef, AsMut)]
pub struct Point(DVec3);

impl Interpolatable for Point {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self(self.0.lerp(target.0, t))
    }
}
