use glam::DVec3;
use std::ops::{Deref, DerefMut};

use crate::prelude::Interpolatable;

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Point(DVec3);

impl From<DVec3> for Point {
    fn from(value: DVec3) -> Self {
        Self(value)
    }
}

impl Deref for Point {
    type Target = DVec3;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Point {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Interpolatable for Point {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self(self.0.lerp(target.0, t))
    }
}

// MARK: Transform3d
impl AsRef<DVec3> for Point {
    fn as_ref(&self) -> &DVec3 {
        &self.0
    }
}
impl AsMut<DVec3> for Point {
    fn as_mut(&mut self) -> &mut DVec3 {
        &mut self.0
    }
}
