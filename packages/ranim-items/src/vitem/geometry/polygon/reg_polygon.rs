use ranim_core::{
    Extract,
    color::{AlphaColor, Srgb},
    core_item::{CoreItem, vitem::DEFAULT_STROKE_WIDTH},
    glam::DVec3,
    proj::CoordinateSystem,
    traits::{
        Aabb, FillColor, LocalCoordinate, Origin, Rotate, RotateLocal, ScaleUniform,
        ScaleUniformByOrigin, ScaleUniformLocal, Shift, StrokeColor, With,
    },
};
use std::f64::consts::TAU;

use crate::vitem::geometry::Polygon;

#[derive(Debug, Clone)]
/// A regular polygon.
pub struct RegPolygon {
    /// Local coordinate system
    pub coord: CoordinateSystem,
    /// Number of sides
    pub sides: usize,
    /// Radius of the polygon (i.e. distance from center to a vertex)
    pub radius: f64,
    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
    /// Fill rgba
    pub fill_rgba: AlphaColor<Srgb>,
}

impl RegPolygon {
    /// Creates a new regular polygon.
    pub fn new(sides: usize, radius: f64) -> Self {
        Self {
            coord: CoordinateSystem::default(),
            sides,
            radius,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
    /// Returns the vertices of the polygon.
    pub fn points(&self) -> Vec<DVec3> {
        let u = self.coord.basis_u();
        let w = self.coord.normal();
        let &Self { sides, radius, .. } = self;
        let center = self.coord.origin;
        (0..sides)
            .map(|i| TAU * (i as f64 / sides as f64))
            .map(|angle| u.rotate_axis(w, angle) * radius + center)
            .collect()
    }
}

impl Origin for RegPolygon {
    fn origin(&self) -> DVec3 {
        self.coord.origin
    }

    fn move_to(&mut self, origin: DVec3) -> &mut Self {
        self.coord.origin = origin;
        self
    }
}

impl LocalCoordinate for RegPolygon {
    fn coord(&self) -> CoordinateSystem {
        self.coord
    }
}

impl Aabb for RegPolygon {
    fn aabb(&self) -> [DVec3; 2] {
        self.points().aabb()
    }
}

impl Shift for RegPolygon {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.coord.shift(offset);
        self
    }
}

impl Rotate for RegPolygon {
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self {
        self.coord.rotate_at_point(angle, axis, point);
        self
    }
}

impl RotateLocal for RegPolygon {}

impl ScaleUniform for RegPolygon {
    fn scale_uniform_at_point(&mut self, scale: f64, point: DVec3) -> &mut Self {
        self.coord.origin.scale_uniform_at_point(scale, point);
        self.radius *= scale;
        self
    }
}

impl ScaleUniformByOrigin for RegPolygon {
    fn scale_uniform(&mut self, scale: f64) -> &mut Self {
        self.radius *= scale;
        self
    }
}

impl ScaleUniformLocal for RegPolygon {}

impl FillColor for RegPolygon {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.fill_rgba
    }

    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.fill_rgba = color;
        self
    }

    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self
    }
}

impl StrokeColor for RegPolygon {
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

impl From<RegPolygon> for Polygon {
    fn from(value: RegPolygon) -> Self {
        Polygon::new(value.points()).with(|item| {
            item.fill_rgba = value.fill_rgba;
            item.stroke_rgba = value.stroke_rgba;
            item.stroke_width = value.stroke_width;
        })
    }
}

impl Extract for RegPolygon {
    type Target = CoreItem;

    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        Polygon::from(self.clone()).extract_into(buf);
    }
}
