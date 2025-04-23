use glam::DVec3;
use ranim_macros::{
    Alignable, BoundingBox, Empty, Fill, Interpolatable, Opacity, Position, Stroke,
};

use super::VItem;

#[derive(Clone, Interpolatable, Alignable, Opacity, Empty, Stroke, Fill, BoundingBox, Position)]
pub struct Line(pub VItem);

impl Line {
    pub fn new(start: DVec3, end: DVec3) -> Self {
        Self(VItem::from_vpoints(vec![start, (start + end) / 2.0, end]))
    }
    pub fn put_start_and_end_on(&mut self, start: DVec3, end: DVec3) -> &mut Self {
        self.0.put_start_and_end_on(start, end);
        self
    }
}
