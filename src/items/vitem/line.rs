use glam::DVec3;
use ranim_macros::{Alignable, BoundingBox, Empty, Fill, Interpolatable, Opacity, Stroke};

use crate::{components::Anchor, traits::Position};

use super::VItem;

#[derive(Clone, Interpolatable, Alignable, Opacity, Empty, Stroke, Fill, BoundingBox)]
pub struct Line(pub VItem);

impl Position for Line {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.0.shift(shift);
        self
    }

    fn rotate_by_anchor(&mut self, angle: f64, axis: DVec3, anchor: Anchor) -> &mut Self {
        self.0.rotate_by_anchor(angle, axis, anchor);
        self
    }

    fn scale_by_anchor(&mut self, scale: DVec3, anchor: Anchor) -> &mut Self {
        self.0.scale_by_anchor(scale, anchor);
        self
    }
}

impl Line {
    pub fn new(start: DVec3, end: DVec3) -> Self {
        Self(VItem::from_vpoints(vec![start, (start + end) / 2.0, end]))
    }
}
