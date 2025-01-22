use std::ops::{Deref, DerefMut};

use glam::Vec3;

use crate::prelude::Alignable;

use super::{ComponentData, Transform3d};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point(Vec3);

impl Default for Point {
    fn default() -> Self {
        Vec3::ZERO.into()
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

impl Transform3d for Point {
    fn position(&self) -> Vec3 {
        self.0
    }

    fn position_mut(&mut self) -> &mut Vec3 {
        &mut self.0
    }
}

impl Alignable for ComponentData<Point> {
    fn is_aligned(&self, other: &Self) -> bool {
        self.len() == other.len()
    }
    fn align_with(&mut self, other: &mut Self) {
        if self.len() < other.len() {
            self.resize_with_last(other.len());
        } else {
            other.resize_with_last(self.len());
        }
    }
}

#[cfg(test)]
mod test {
    #[allow(unused)]
    use super::*;

}
