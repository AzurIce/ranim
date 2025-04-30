use glam::DVec3;
use ranim_macros::{
    Alignable, BoundingBox, Empty, Fill, Interpolatable, Opacity, Position, Stroke,
};

use crate::render::primitives::{vitem::VItemPrimitive, Extract, Primitive};

use super::VItem;

#[derive(Clone, Interpolatable, Alignable, Opacity, Empty, Stroke, Fill, BoundingBox, Position)]
pub struct Line(pub VItem);

impl Line {
    pub fn points(&self) -> [DVec3; 2] {
        [
            *self.0.get_anchor(0).unwrap(),
            *self.0.get_anchor(1).unwrap(),
        ]
    }
    pub fn center(&self) -> DVec3 {
        let [p1, p2] = self.points();
        (p1 + p2) / 2.0
    }
    pub fn new(p1: DVec3, p2: DVec3) -> Self {
        Self(VItem::from_vpoints(vec![p1, (p1 + p2) / 2.0, p1]))
    }
    pub fn put_start_and_end_on(&mut self, start: DVec3, end: DVec3) -> &mut Self {
        self.0.put_start_and_end_on(start, end);
        self
    }
}

impl Extract for Line {
    type Primitive = VItemPrimitive;
    fn extract(&self) -> <VItemPrimitive as Primitive>::Data {
        self.0.extract()
    }
}