use glam::{vec2, vec3, Vec2, Vec3};

use crate::rabject::{
    vmobject::{TransformAnchor, VMobject},
    Blueprint, RabjectWithId,
};

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

impl Blueprint<VMobject> for Arc {
    fn build(self) -> RabjectWithId<VMobject> {
        const NUM_SEGMENTS: usize = 8;
        let len = 2 * NUM_SEGMENTS + 1;

        let angle_step = self.angle / (len - 1) as f32;
        let mut points = (0..len)
            .map(|i| {
                let angle = i as f32 * angle_step;
                vec2(angle.cos() as f32, angle.sin() as f32).extend(0.0) * self.radius
            })
            .collect::<Vec<_>>();

        let theta = self.angle / NUM_SEGMENTS as f32;
        points.iter_mut().skip(1).step_by(2).for_each(|p| {
            *p /= (theta / 2.0).cos();
        });
        // trace!("start: {:?}, end: {:?}", points[0], points[len - 1]);
        let mut vmobject = VMobject::from_points(points);
        vmobject.set_stroke_width(self.stroke_width);
        vmobject.into()
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

impl Blueprint<VMobject> for ArcBetweenPoints {
    fn build(self) -> RabjectWithId<VMobject> {
        let radius = (self.start.distance(self.end) / 2.0) / self.angle.sin();
        let arc = Arc::new(self.angle)
            .with_radius(radius)
            .with_stroke_width(self.stroke_width);
        let mut mobject = arc.build();
        mobject.put_start_and_end_on(self.start, self.end);
        mobject
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

impl Blueprint<VMobject> for Circle {
    fn build(self) -> RabjectWithId<VMobject> {
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

impl Blueprint<VMobject> for Dot {
    fn build(self) -> RabjectWithId<VMobject> {
        let mut mobject = Circle::new(self.radius)
            .with_stroke_width(self.stroke_width)
            .build();
        mobject.shift(self.point);
        mobject
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

impl Blueprint<VMobject> for Ellipse {
    fn build(self) -> RabjectWithId<VMobject> {
        let mut mobject = Circle::new(self.width)
            .with_stroke_width(self.stroke_width)
            .build();
        mobject.scale(
            vec3(self.width, self.height, 1.0),
            TransformAnchor::origin(),
        );
        mobject
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

impl Blueprint<VMobject> for Polygon {
    fn build(self) -> RabjectWithId<VMobject> {
        // TODO: Handle 0 len
        if self.corner_points.len() == 0 {
            return VMobject::from_points(vec![]).into();
        }

        let vertices = self
            .corner_points
            .into_iter()
            .map(|v| v.extend(0.0))
            .collect::<Vec<_>>();

        let mut mobject = VMobject::from_corner_points(vertices);
        mobject.set_stroke_width(self.stroke_width);
        mobject.into()
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

impl Blueprint<VMobject> for Rect {
    fn build(self) -> RabjectWithId<VMobject> {
        let mobject = Polygon::new(vec![
            vec2(0.0, 0.0),
            vec2(self.width, 0.0),
            vec2(self.width, self.height),
            vec2(0.0, self.height),
        ])
        .with_stroke_width(self.stroke_width)
        .build();
        mobject.into()
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

impl Blueprint<VMobject> for Square {
    fn build(self) -> RabjectWithId<VMobject> {
        Rect::new(self.side_length, self.side_length)
            .with_stroke_width(self.stroke_width)
            .build()
    }
}
