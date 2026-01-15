use ranim_core::{Extract, core_item::CoreItem, glam::DVec3};
use ranim_macros::{
    Alignable, BoundingBox, Empty, Fill, Interpolatable, Opacity, Partial, Position, Stroke,
};

use super::VItem;

// #[derive(Clone)]
// pub struct NumberLine {
//     pub arrow: Arrow,
// }

#[derive(Clone, Interpolatable, Alignable, Opacity, Empty, Partial)]
pub struct Line(pub VItem);

impl From<Line> for VItem {
    fn from(value: Line) -> Self {
        value.0
    }
}

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
        Self(VItem::from_vpoints(vec![p1, (p1 + p2) / 2.0, p2]))
    }
    pub fn start(&self) -> DVec3 {
        self.points()[0]
    }
    pub fn end(&self) -> DVec3 {
        self.points()[1]
    }
    pub fn put_start_on(&mut self, pos: DVec3) -> &mut Self {
        let start = self.points()[0];
        self.put_start_and_end_on(start, pos)
    }
    pub fn put_end_on(&mut self, pos: DVec3) -> &mut Self {
        let end = self.points()[1];
        self.put_start_and_end_on(pos, end)
    }
    pub fn put_start_and_end_on(&mut self, start: DVec3, end: DVec3) -> &mut Self {
        self.0.put_start_and_end_on(start, end);
        self
    }
}

impl Extract for Line {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        VItem::from(self.clone()).extract_into(buf);
    }
}
