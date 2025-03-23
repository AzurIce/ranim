use std::ops::{Deref, DerefMut};

use glam::Vec3;

use crate::prelude::Interpolatable;

use super::Transform3dComponent;

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Point(Vec3);

impl Transform3dComponent for Point {
    fn pos(&self) -> Vec3 {
        self.0
    }

    fn iter_points(&self) -> impl Iterator<Item = &Vec3> {
        std::iter::once(&self.0)
    }
    fn iter_points_mut(&mut self) -> impl Iterator<Item = &mut Vec3> {
        std::iter::once(&mut self.0)
    }
}


impl From<Vec3> for Point {
    fn from(value: Vec3) -> Self {
        Self(value)
    }
}

impl Deref for Point {
    type Target = Vec3;

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
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Self(self.0.lerp(target.0, t))
    }
}

// MARK: Transform3d
impl AsRef<Vec3> for Point {
    fn as_ref(&self) -> &Vec3 {
        &self.0
    }
}
impl AsMut<Vec3> for Point {
    fn as_mut(&mut self) -> &mut Vec3 {
        &mut self.0
    }
}
