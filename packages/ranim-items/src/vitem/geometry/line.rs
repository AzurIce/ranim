use ranim_core::{
    components::vpoint::VPointVec,
    glam::DVec3,
    traits::{Aabb, Anchor, RotateTransform, ScaleTransform, ShiftTransform},
};
use ranim_macros::Interpolatable;

use crate::vitem::{VItem, VPath};

/// A line segment.
#[derive(Debug, Clone, Interpolatable)]
pub struct Line {
    /// The start and end points of the line.
    pub points: [DVec3; 2],
    /// The distance two endpoints extends or shrinks from its original position.
    /// Positive value means extension and negative value means shrinking.
    pub extrude: [f64; 2],
}

impl VItem<Line> {
    /// Creates a new line segment with the given start and end points.
    pub fn new(start: DVec3, end: DVec3) -> Self {
        Self::new_with(Line {
            points: [start, end],
            extrude: [0., 0.],
        })
    }
}

impl Line {

    /// Inverts the direction of the line segment.
    pub fn invert(&mut self) -> &mut Self {
        self.points.reverse();
        self.extrude.reverse();
        self
    }

    /// Returns the start and end points of the line segment considering the extrusion distance.
    pub fn points_with_extrude(&self) -> [DVec3; 2] {
        let [p1, p2] = self.points;
        let [ext1, ext2] = self.extrude;
        let d = (p2 - p1).normalize();
        [p1 - d * ext1, p2 + d * ext2]
    }
}

impl VPath for Line {
    fn normal(&self) -> DVec3 {
        DVec3::Z
    }
    fn build_vpoint_vec(&self) -> VPointVec {
        let [p1, p2] = self.points_with_extrude();
        VPointVec(vec![p1, (p1 + p2) / 2., p2])
    }
}

impl Anchor<Line> for f64 {
    fn locate_on(&self, target: &Line) -> DVec3 {
        let [p1, p2] = target.points;
        p1.lerp(p2, *self)
    }
}

impl Aabb for Line {
    fn aabb(&self) -> [DVec3; 2] {
        self.points_with_extrude().aabb()
    }
}

impl ShiftTransform for Line {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.points.shift(offset);
        self
    }
}

impl RotateTransform for Line {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.points.rotate_on_axis(axis, angle);
        self
    }
}

impl ScaleTransform for Line {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        let [p1, p2] = self.points;
        let k = ((p2 - p1).normalize() * scale).length();
        self.points.scale(scale);
        self.extrude.iter_mut().for_each(|e| *e *= k);
        self
    }
}

// impl Extract for Line {
//     type Target = CoreItem;

//     fn extract_into(&self, buf: &mut Vec<Self::Target>) {
//         use crate::vitem::VItem;
//         VItem::new(self.clone()).extract_into(buf);
//     }
// }
