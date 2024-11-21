use bezier_rs::Bezier;
use glam::{vec2, vec3, vec4, Vec2, Vec3, Vec4};
use itertools::Itertools;
use palette::{rgb, Srgba};
// use log::trace;

use crate::{
    pipeline::simple,
    renderer::vmobject::{VMobjectRenderer, VMobjectVertex},
    utils::{beziers_to_fill, beziers_to_stroke, SubpathWidth},
};

use super::{Mobject, ToMobject, TransformAnchor};

/// A part of a circle
// #[mobject(SimplePipeline)]
#[derive(Debug, Clone)]
pub struct Arc {
    /// Angle in radians of the arc
    pub angle: f32,
    pub radius: f32,
    pub stroke_width: SubpathWidth,
}

impl Arc {
    pub fn new(angle: f32) -> Self {
        Self {
            angle,
            radius: 1.0,
            stroke_width: SubpathWidth::default(),
        }
    }
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
    pub fn with_stroke_width(mut self, stroke_width: SubpathWidth) -> Self {
        self.stroke_width = stroke_width;
        self
    }
}

impl ToMobject for Arc {
    type Renderer = VMobjectRenderer;

    fn to_mobject(self) -> Mobject<VMobjectVertex> {
        const NUM_SEGMENTS: usize = 8;
        let len = 2 * NUM_SEGMENTS + 1;

        let angle_step = self.angle / (len - 1) as f32;
        let mut points = (0..len)
            .map(|i| {
                let angle = i as f32 * angle_step;
                vec2(angle.cos() as f32, angle.sin() as f32) * self.radius
            })
            .collect::<Vec<_>>();

        let theta = self.angle / NUM_SEGMENTS as f32;
        points.iter_mut().skip(1).step_by(2).for_each(|p| {
            *p /= (theta / 2.0).cos();
        });
        // trace!("start: {:?}, end: {:?}", points[0], points[len - 1]);

        // let beziers = points
        //     .iter()
        //     .step_by(2)
        //     .zip(points.iter().skip(1).step_by(2))
        //     .zip(points.iter().skip(2).step_by(2))
        //     .map(|((&p1, &p2), &p3)| {
        //         let [p1, p2, p3] = [p1 * self.radius, p2 * self.radius, p3 * self.radius];
        //         Bezier::from_quadratic_dvec2(p1.as_dvec2(), p2.as_dvec2(), p3.as_dvec2())
        //     })
        //     .collect::<Vec<_>>();

        // trace!("beziers: {:?}", beziers.len());
        // Mobject::new::<VMobjectRenderer>(beziers_to_stroke(
        //     beziers,
        //     self.stroke_width,
        //     self.angle == std::f32::consts::TAU,
        // ))
        if self.angle == std::f32::consts::TAU {
            BezierShape::closed(points)
        } else {
            BezierShape::unclosed(points)
        }
        .with_width(self.stroke_width)
        .to_mobject()
    }
}

pub struct ArcBetweenPoints {
    pub start: Vec3,
    pub end: Vec3,
    pub angle: f32,
    pub stroke_width: SubpathWidth,
}

impl ArcBetweenPoints {
    pub fn new(start: Vec3, end: Vec3, angle: f32) -> Self {
        Self {
            start,
            end,
            angle,
            stroke_width: SubpathWidth::default(),
        }
    }
    pub fn with_stroke_width(mut self, stroke_width: SubpathWidth) -> Self {
        self.stroke_width = stroke_width;
        self
    }
}

impl ToMobject for ArcBetweenPoints {
    type Renderer = VMobjectRenderer;

    fn to_mobject(self) -> Mobject<VMobjectVertex> {
        let radius = (self.start.distance(self.end) / 2.0) / self.angle.sin();
        let arc = Arc::new(self.angle)
            .with_radius(radius)
            .with_stroke_width(self.stroke_width);
        let mut mobject = arc.to_mobject();
        mobject.put_start_and_end_on(self.start, self.end);
        mobject
    }
}

pub struct Circle {
    pub radius: f32,
    pub stroke_width: SubpathWidth,
}

impl Circle {
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            stroke_width: SubpathWidth::default(),
        }
    }

    pub fn with_stroke_width(mut self, stroke_width: SubpathWidth) -> Self {
        self.stroke_width = stroke_width;
        self
    }
}

impl ToMobject for Circle {
    type Renderer = VMobjectRenderer;

    fn to_mobject(self) -> Mobject<VMobjectVertex> {
        Arc::new(std::f32::consts::TAU)
            .with_radius(self.radius)
            .with_stroke_width(self.stroke_width)
            .to_mobject()
    }
}

pub struct Dot {
    pub point: Vec3,
    pub radius: f32,
    pub stroke_width: SubpathWidth,
}

impl Dot {
    pub fn new(point: Vec3) -> Self {
        Self {
            point,
            radius: 0.08,
            stroke_width: SubpathWidth::default(),
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

    pub fn with_stroke_width(mut self, stroke_width: SubpathWidth) -> Self {
        self.stroke_width = stroke_width;
        self
    }
}

impl ToMobject for Dot {
    type Renderer = VMobjectRenderer;

    fn to_mobject(self) -> Mobject<VMobjectVertex> {
        let mut mobject = Circle::new(self.radius)
            .with_stroke_width(self.stroke_width)
            .to_mobject();
        mobject.shift(self.point);
        mobject
    }
}

pub struct Ellipse {
    pub width: f32,
    pub height: f32,
    pub stroke_width: SubpathWidth,
}

impl Ellipse {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            stroke_width: SubpathWidth::default(),
        }
    }

    pub fn with_stroke_width(mut self, stroke_width: SubpathWidth) -> Self {
        self.stroke_width = stroke_width;
        self
    }
}

impl ToMobject for Ellipse {
    type Renderer = VMobjectRenderer;

    fn to_mobject(self) -> Mobject<VMobjectVertex> {
        let mut mobject = Circle::new(self.width)
            .with_stroke_width(self.stroke_width)
            .to_mobject();
        mobject.scale(
            vec3(self.width, self.height, 1.0),
            TransformAnchor::origin(),
        );
        mobject
    }
}

#[derive(Debug, Clone)]
pub struct Polygon {
    pub vertices: Vec<Vec2>,
    pub width: SubpathWidth,
}

impl Polygon {
    pub fn new(vertices: Vec<Vec2>) -> Self {
        Self {
            vertices,
            width: SubpathWidth::Middle(1.0),
        }
    }
    pub fn with_width(mut self, width: SubpathWidth) -> Self {
        self.width = width;
        self
    }
}

impl ToMobject for Polygon {
    type Renderer = VMobjectRenderer;

    fn to_mobject(self) -> Mobject<VMobjectVertex> {
        // TODO: Handle 0 len
        if self.vertices.len() == 0 {
            return Mobject::new::<VMobjectRenderer>(vec![]);
        }

        let vertices = self.vertices.clone();

        let anchors = vertices;
        let handles = anchors
            .windows(2)
            .map(|window| 0.5 * (window[0] + window[1]))
            .collect::<Vec<_>>();

        assert_eq!(anchors.len(), handles.len() + 1);

        let points = anchors
            .into_iter()
            .interleave(handles.into_iter())
            .collect::<Vec<_>>();
        // let beziers = points
        //     .iter()
        //     .step_by(2)
        //     .zip(
        //         points
        //             .iter()
        //             .skip(1)
        //             .chain(points.iter().take(1))
        //             .step_by(2),
        //     )
        //     .zip(
        //         points
        //             .iter()
        //             .skip(2)
        //             .chain(points.iter().take(2))
        //             .step_by(2),
        //     )
        //     .map(|((&p1, &p2), &p3)| {
        //         Bezier::from_quadratic_dvec2(p1.as_dvec2(), p2.as_dvec2(), p3.as_dvec2())
        //     })
        //     .collect::<Vec<_>>();
        // println!("beziers: {:?}", beziers.len());
        // Mobject::new::<VMobjectRenderer>(beziers_to_stroke(beziers, self.width, true))
        BezierShape::closed(points)
            .with_width(self.width)
            .to_mobject()
    }
}

pub struct BezierShape {
    pub points: Vec<Vec2>,
    pub width: SubpathWidth,
    pub stroke_color: Vec4,
    pub fill_color: Vec4,
    pub closed: bool,
}

impl BezierShape {
    pub fn closed(points: Vec<Vec2>) -> Self {
        Self {
            closed: true,
            ..Self::unclosed(points)
        }
    }

    pub fn unclosed(points: Vec<Vec2>) -> Self {
        let stroke_color: Srgba = Srgba::from_u32::<rgb::channels::Rgba>(0x29ABCAFF).into();
        Self {
            points,
            width: SubpathWidth::Middle(1.0),
            stroke_color: vec4(
                stroke_color.red,
                stroke_color.green,
                stroke_color.blue,
                stroke_color.alpha,
            ),
            fill_color: Vec4::ZERO,
            closed: false,
        }
    }

    pub fn with_width(mut self, width: SubpathWidth) -> Self {
        self.width = width;
        self
    }

    pub fn with_stroke_color(mut self, stroke_color: Vec4) -> Self {
        self.stroke_color = stroke_color;
        self
    }

    pub fn with_fill_color(mut self, fill_color: Vec4) -> Self {
        self.fill_color = fill_color;
        self
    }
}

impl ToMobject for BezierShape {
    type Renderer = VMobjectRenderer;

    fn to_mobject(self) -> Mobject<<Self::Renderer as crate::renderer::Renderer>::Vertex>
    where
        Self: Sized,
    {
        let points = self.points.clone();

        let anchors = points;
        let handles = anchors
            .windows(2)
            .map(|window| 0.5 * (window[0] + window[1]))
            .collect::<Vec<_>>();

        assert_eq!(anchors.len(), handles.len() + 1);

        let points = anchors
            .into_iter()
            .interleave(handles.into_iter())
            .collect::<Vec<_>>();

        let beziers = points
            .iter()
            .step_by(2)
            .zip(
                points
                    .iter()
                    .skip(1)
                    .chain(points.iter().take(1))
                    .step_by(2),
            )
            .zip(
                points
                    .iter()
                    .skip(2)
                    .chain(points.iter().take(2))
                    .step_by(2),
            )
            .map(|((&p1, &p2), &p3)| {
                Bezier::from_quadratic_dvec2(p1.as_dvec2(), p2.as_dvec2(), p3.as_dvec2())
            })
            .collect::<Vec<_>>();

        let beziers = beziers
            .into_iter()
            .filter(|bezier| !bezier.is_point())
            .collect::<Vec<_>>();

        let mut vertices = beziers_to_stroke(&beziers, self.width, self.stroke_color, self.closed);

        if self.closed {
            vertices.extend(beziers_to_fill(&beziers, self.fill_color).into_iter());
        }

        Mobject::new::<VMobjectRenderer>(vertices)
    }
}
