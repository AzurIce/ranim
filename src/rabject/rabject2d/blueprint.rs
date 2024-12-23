use glam::{vec2, vec3, Vec2, Vec3};

use super::{vpath::{blueprint::*, VPath}, RabjectEntity2d};
use crate::rabject::{Blueprint, TransformAnchor};

/// A part of a circle
// #[mobject(SimplePipeline)]
#[derive(Debug, Clone)]
pub struct Arc {
    /// Angle in radians of the arc
    pub angle: f32,
    pub radius: f32,
    pub stroke_width: f32,
}

impl Arc {
    pub fn new(angle: f32) -> Self {
        Self {
            angle,
            radius: 1.0,
            stroke_width: 10.0,
        }
    }
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
    pub fn with_stroke_width(mut self, stroke_width: f32) -> Self {
        self.stroke_width = stroke_width;
        self
    }
}

impl Blueprint<RabjectEntity2d<VPath>> for Arc {
    fn build(self) -> RabjectEntity2d<VPath> {
        const NUM_SEGMENTS: usize = 8;
        let len = 2 * NUM_SEGMENTS + 1;

        let mut points = (0..len)
            .map(|i| {
                let angle = self.angle * i as f32 / (len - 1) as f32;
                let (mut x, mut y) = (angle.cos(), angle.sin());
                if x.abs() < 1.8e-7 {
                    x = 0.0;
                }
                if y.abs() < 1.8e-7 {
                    y = 0.0;
                }
                vec2(x, y).extend(0.0) * self.radius
            })
            .collect::<Vec<_>>();

        let theta = self.angle / NUM_SEGMENTS as f32;
        points.iter_mut().skip(1).step_by(2).for_each(|p| {
            *p /= (theta / 2.0).cos();
        });

        let mut builder = points
            .iter()
            .skip(1)
            .step_by(2)
            .zip(points.iter().skip(2).step_by(2))
            .fold(VPathBuilder::start(points[0]), |builder, (h, p)| {
                builder.quad_to(*p, *h)
            });

        if self.angle == std::f32::consts::TAU {
            builder = builder.close();
        }

        // trace!("start: {:?}, end: {:?}", points[0], points[len - 1]);
        let mut vmobject = builder.build();
        vmobject.set_stroke_width(self.stroke_width);
        vmobject
    }
}

pub struct ArcBetweenPoints {
    pub start: Vec3,
    pub end: Vec3,
    pub angle: f32,
    pub stroke_width: f32,
}

impl ArcBetweenPoints {
    pub fn new(start: Vec3, end: Vec3, angle: f32) -> Self {
        Self {
            start,
            end,
            angle,
            stroke_width: 10.0,
        }
    }
    pub fn with_stroke_width(mut self, stroke_width: f32) -> Self {
        self.stroke_width = stroke_width;
        self
    }
}

impl Blueprint<RabjectEntity2d<VPath>> for ArcBetweenPoints {
    fn build(self) -> RabjectEntity2d<VPath> {
        let radius = (self.start.distance(self.end) / 2.0) / self.angle.sin();
        let arc = Arc::new(self.angle)
            .with_radius(radius)
            .with_stroke_width(self.stroke_width);
        let mut vpath = arc.build();
        vpath.put_start_and_end_on(self.start, self.end);
        vpath
    }
}

pub struct Circle {
    pub radius: f32,
    pub stroke_width: f32,
}

impl Circle {
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            stroke_width: 10.0,
        }
    }

    pub fn with_stroke_width(mut self, stroke_width: f32) -> Self {
        self.stroke_width = stroke_width;
        self
    }
}

impl Blueprint<RabjectEntity2d<VPath>> for Circle {
    fn build(self) -> RabjectEntity2d<VPath> {
        Arc::new(std::f32::consts::TAU)
            .with_radius(self.radius)
            .with_stroke_width(self.stroke_width)
            .build()
    }
}

pub struct Dot {
    pub point: Vec3,
    pub radius: f32,
    pub stroke_width: f32,
}

impl Dot {
    pub fn new(point: Vec3) -> Self {
        Self {
            point,
            radius: 0.08,
            stroke_width: 10.0,
        }
    }

    pub fn small(mut self) -> Self {
        self.radius = 0.04;
        self
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn with_stroke_width(mut self, stroke_width: f32) -> Self {
        self.stroke_width = stroke_width;
        self
    }
}

impl Blueprint<RabjectEntity2d<VPath>> for Dot {
    fn build(self) -> RabjectEntity2d<VPath> {
        let mut vpath = Circle::new(self.radius)
            .with_stroke_width(self.stroke_width)
            .build();
        vpath.shift(self.point);
        vpath
    }
}

pub struct Ellipse {
    pub width: f32,
    pub height: f32,
    pub stroke_width: f32,
}

impl Ellipse {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            stroke_width: 10.0,
        }
    }

    pub fn with_stroke_width(mut self, stroke_width: f32) -> Self {
        self.stroke_width = stroke_width;
        self
    }
}

impl Blueprint<RabjectEntity2d<VPath>> for Ellipse {
    fn build(self) -> RabjectEntity2d<VPath> {
        let mut vpath = Circle::new(1.0)
            .with_stroke_width(self.stroke_width)
            .build();
        vpath.scale(
            vec3(self.width, self.height, 1.0),
            TransformAnchor::origin(),
        );
        vpath
    }
}

#[derive(Debug, Clone)]
pub struct Polygon {
    pub corner_points: Vec<Vec2>,
    pub stroke_width: f32,
}

impl Polygon {
    pub fn new(corner_points: Vec<Vec2>) -> Self {
        Self {
            corner_points,
            stroke_width: 10.0,
        }
    }
    pub fn with_stroke_width(mut self, stroke_width: f32) -> Self {
        self.stroke_width = stroke_width;
        self
    }
}

impl Blueprint<RabjectEntity2d<VPath>> for Polygon {
    fn build(self) -> RabjectEntity2d<VPath> {
        // TODO: Handle
        assert!(self.corner_points.len() >= 3);

        let vertices = self
            .corner_points
            .into_iter()
            .map(|v| v.extend(0.0))
            .collect::<Vec<_>>();

        let mut vpath = vertices[1..]
            .iter()
            .fold(VPathBuilder::start(vertices[0]), |builder, &v| {
                builder.line_to(v)
            })
            .close()
            .build();

        vpath.set_stroke_width(self.stroke_width);
        vpath
    }
}

pub struct Rect {
    pub width: f32,
    pub height: f32,
    pub stroke_width: f32,
}

impl Rect {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            stroke_width: 10.0,
        }
    }
    pub fn with_stroke_width(mut self, stroke_width: f32) -> Self {
        self.stroke_width = stroke_width;
        self
    }
}

impl Blueprint<RabjectEntity2d<VPath>> for Rect {
    fn build(self) -> RabjectEntity2d<VPath> {
        Polygon::new(vec![
            vec2(0.0, 0.0),
            vec2(self.width, 0.0),
            vec2(self.width, self.height),
            vec2(0.0, self.height),
        ])
        .with_stroke_width(self.stroke_width)
        .build()
    }
}

pub struct Square {
    pub side_length: f32,
    pub stroke_width: f32,
}

impl Square {
    pub fn new(side_length: f32) -> Self {
        Self {
            side_length,
            stroke_width: 10.0,
        }
    }
    pub fn with_stroke_width(mut self, stroke_width: f32) -> Self {
        self.stroke_width = stroke_width;
        self
    }
}

impl Blueprint<RabjectEntity2d<VPath>> for Square {
    fn build(self) -> RabjectEntity2d<VPath> {
        Rect::new(self.side_length, self.side_length)
            .with_stroke_width(self.stroke_width)
            .build()
    }
}

#[cfg(test)]
mod test {
    use crate::rabject::Blueprint;

    use super::*;

    #[test]
    fn test_arc() {
        let arc = Arc::new(std::f32::consts::PI).build();
        assert!(!arc.is_closed());

        let arc = Arc::new(std::f32::consts::TAU).build();
        assert_eq!(arc.points().first(), arc.points().last());
        assert!(arc.is_closed());
    }
}
