use ranim_core::{
    Extract,
    color::{AlphaColor, Srgb},
    core_item::{CoreItem, vitem::DEFAULT_STROKE_WIDTH},
    glam::DVec3,
    traits::{
        Aabb, Discard as _, Locate, Opacity, RotateTransform, ScaleTransform, ShiftTransform,
        StrokeColor, StrokeWidth, With as _,
    },
};
use ranim_macros::Interpolatable;

use crate::vitem::VItem;

/// A line segment.
#[derive(Debug, Clone, Interpolatable)]
pub struct Line {
    /// The start and end points of the line.
    pub points: [DVec3; 2],
    /// The distance two endpoints extends or shrinks from its original position.
    /// Positive value means extension and negative value means shrinking.
    pub extrude: [f64; 2],
    /// Stroke RGBA values.
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width.
    pub stroke_width: f32,
}

impl Line {
    /// Creates a new line segment with the given start and end points.
    pub fn new(start: DVec3, end: DVec3) -> Self {
        Self {
            points: [start, end],
            extrude: [0., 0.],
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
        }
    }

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

impl Locate<Line> for f64 {
    fn locate(&self, target: &Line) -> DVec3 {
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

impl StrokeColor for Line {
    fn stroke_color(&self) -> AlphaColor<Srgb> {
        self.stroke_rgba
    }

    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgba = self.stroke_rgba.with_alpha(opacity);
        self
    }

    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.stroke_rgba = color;
        self
    }
}

impl From<Line> for VItem {
    fn from(value: Line) -> Self {
        let [p1, p2] = value.points_with_extrude();
        VItem::from_vpoints(vec![p1, (p1 + p2) / 2., p2]).with(|item| {
            item.set_stroke_color(value.stroke_rgba)
                .set_stroke_width(value.stroke_width)
                .discard()
        })
    }
}

impl Opacity for Line {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.set_stroke_opacity(opacity);
        self
    }
}

impl Extract for Line {
    type Target = CoreItem;

    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        VItem::from(self.clone()).extract_into(buf);
    }
}
