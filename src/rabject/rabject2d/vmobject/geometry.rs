use core::f32;

use glam::{vec2, Vec2};
use vello::kurbo::{self, PathEl};

use crate::rabject::{
    rabject2d::bez_path::{BezPath, FillOptions, StrokeOptions},
    Blueprint,
};

use super::VMobject;

/// A part of a circle
pub struct Arc {
    pub angle: f32,
    pub radius: f32,
    pub x_rotation: f32,
    pub stroke_width: f32,
}

impl Arc {
    pub fn new(angle: f32) -> Self {
        Self {
            angle,
            ..Default::default()
        }
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn with_stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }
}

impl Default for Arc {
    fn default() -> Self {
        Self {
            angle: 0.0,
            radius: 1.0,
            x_rotation: 0.0,
            stroke_width: 4.0,
        }
    }
}

impl Blueprint<VMobject> for Arc {
    fn build(self) -> VMobject {
        // when x_rotation is 0.0, the arc starts from (radius, 0.0) and goes clockwise
        let start = (
            self.radius * self.x_rotation.cos(),
            self.radius * self.x_rotation.sin(),
        );

        let path = kurbo::BezPath::from_vec(
            [kurbo::PathEl::MoveTo(
                (start.0 as f64, start.1 as f64).into(),
            )]
            .into_iter()
            .chain(
                kurbo::Arc::new(
                    (0.0, 0.0),
                    (self.radius as f64, self.radius as f64),
                    0.0,
                    self.angle as f64,
                    0.0, // std::f64::consts::PI / 2.0,
                )
                .append_iter(0.1),
            )
            .collect(),
        );

        let stroke = StrokeOptions::default();
        let fill = FillOptions::default();

        let mut path = BezPath {
            inner: path,
            stroke,
            fill,
        };

        path.set_stroke_width(self.stroke_width).set_fill_alpha(0.0);

        VMobject::new(vec![path])
    }
}

impl VMobject {
    pub fn blueprint_arc(angle: f32) -> Arc {
        Arc::new(angle)
    }
}

/// An Arc between two points and specific angle
pub struct ArcBetweenPoints {
    pub start: Vec2,
    pub end: Vec2,
    pub angle: f32,
    pub stroke_width: f32,
}

impl ArcBetweenPoints {
    pub fn new(start: Vec2, end: Vec2, angle: f32) -> Self {
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
    fn build(self) -> VMobject {
        let radius = (self.start.distance(self.end) / 2.0)
            / if self.angle > f32::consts::PI {
                self.angle - f32::consts::PI
            } else {
                self.angle
            }
            .sin();
        let mut vmobject = VMobject::blueprint_arc(self.angle)
            .with_radius(radius)
            .with_stroke_width(self.stroke_width)
            .build();
        vmobject.put_start_and_end_on(self.start, self.end);
        vmobject
    }
}

impl VMobject {
    pub fn blueprint_arc_between_points(start: Vec2, end: Vec2, angle: f32) -> ArcBetweenPoints {
        ArcBetweenPoints::new(start, end, angle)
    }
}

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
    fn build(self) -> VMobject {
        // TODO: Handle
        assert!(self.corner_points.len() >= 3);

        let path: kurbo::BezPath = [PathEl::MoveTo(
            kurbo::Point::new(
                self.corner_points[0].x as f64,
                self.corner_points[0].y as f64,
            )
            .into(),
        )]
        .into_iter()
        .chain(
            self.corner_points
                .iter()
                .skip(1)
                .map(|p| PathEl::LineTo((p.x as f64, p.y as f64).into())),
        )
        .chain([PathEl::ClosePath].into_iter())
        .collect();

        let mut path = BezPath {
            inner: path,
            stroke: StrokeOptions::default(),
            fill: FillOptions::default(),
        };
        path.set_stroke_width(self.stroke_width);
        VMobject::new(vec![path])
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
    fn build(self) -> VMobject {
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

impl Blueprint<VMobject> for Square {
    fn build(self) -> VMobject {
        Rect::new(self.side_length, self.side_length)
            .with_stroke_width(self.stroke_width)
            .build()
    }
}

#[cfg(test)]
mod test {
    use crate::rabject::Blueprint;

    use super::*;

    // #[test]
    // fn test_arc() {
    //     let arc = Arc::new(std::f32::consts::PI).build();
    //     assert!(!arc.is_closed());

    //     let arc = Arc::new(std::f32::consts::TAU).build();
    //     assert_eq!(arc.points().first(), arc.points().last());
    //     assert!(arc.is_closed());
    // }
}
