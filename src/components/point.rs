use derive_more::{AsMut, AsRef, Deref, DerefMut, From};
use glam::DVec3;
use serde::{Deserialize, Serialize};

use crate::prelude::Interpolatable;

#[derive(Default, Debug, Clone, Copy, PartialEq, Deref, DerefMut, From, AsRef, AsMut)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Point(DVec3);

impl Interpolatable for Point {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self(self.0.lerp(target.0, t))
    }
}
