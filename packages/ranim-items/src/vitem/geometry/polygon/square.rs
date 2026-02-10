use std::f64::consts::PI;

use ranim_core::{
    Extract,
    anchor::Aabb,
    color,
    core_item::CoreItem,
    glam,
    proj::CoordinateSystem,
    traits::{
        LocalCoordinate, Origin, Rotate, RotateLocal, ScaleUniform, ScaleUniformByOrigin,
        ScaleUniformLocal, Shift, With,
    },
};

use color::{AlphaColor, Srgb};
use glam::{DVec3, dvec2};

use super::{Polygon, Rectangle};
use crate::vitem::{DEFAULT_STROKE_WIDTH, VItem, geometry::RegPolygon};
use ranim_core::traits::{Alignable, FillColor, Opacity, StrokeColor};

// MARK: ### Square ###
/// A Square
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Square {
    /// Local coordinate system
    pub coord: CoordinateSystem,
    /// Size
    pub size: f64,

    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
    /// Fill rgba
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Square {
    /// Constructor
    pub fn new(size: f64) -> Self {
        Self {
            coord: CoordinateSystem::default(),
            size,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
}

// MARK: Traits impl
impl Origin for Square {
    fn origin(&self) -> DVec3 {
        self.coord.origin
    }

    fn move_to(&mut self, origin: DVec3) -> &mut Self {
        self.coord.origin = origin;
        self
    }
}
impl LocalCoordinate for Square {
    fn coord(&self) -> ranim_core::proj::CoordinateSystem {
        self.coord
    }
}

impl ScaleUniform for Square {
    fn scale_uniform_at_point(&mut self, scale: f64, point: DVec3) -> &mut Self {
        self.coord.origin.scale_uniform_at_point(scale, point);
        self.size *= scale;
        self
    }
}

impl ScaleUniformByOrigin for Square {
    fn scale_uniform(&mut self, scale: f64) -> &mut Self {
        self.size *= scale;
        self
    }
}

impl ScaleUniformLocal for Square {}

impl Aabb for Square {
    fn aabb(&self) -> [DVec3; 2] {
        let (u, v) = self.coord.basis();
        let center = self.origin();
        [
            center + self.size / 2. * (u + v),
            center - self.size / 2. * (u + v),
        ]
        .aabb()
    }
}

impl Shift for Square {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.coord.shift(shift);
        self
    }
}

impl Rotate for Square {
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self {
        self.coord.rotate_at_point(angle, axis, point);
        self
    }
}

impl RotateLocal for Square {}

impl Alignable for Square {
    fn is_aligned(&self, _other: &Self) -> bool {
        true
    }
    fn align_with(&mut self, _other: &mut Self) {}
}

impl Opacity for Square {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgba = self.stroke_rgba.with_alpha(opacity);
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self
    }
}

impl StrokeColor for Square {
    fn stroke_color(&self) -> AlphaColor<Srgb> {
        self.stroke_rgba
    }
    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.stroke_rgba = color;
        self
    }
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgba = self.stroke_rgba.with_alpha(opacity);
        self
    }
}

impl FillColor for Square {
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

impl Extract for Square {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        VItem::from(self.clone()).extract_into(buf);
    }
}

// MARK: Conversions
impl From<Square> for Rectangle {
    fn from(value: Square) -> Self {
        let Square {
            coord,
            size: width,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        } = value;
        let (u, v) = coord.basis();
        let center = coord.origin;
        let p0 = center - width / 2.0 * u - width / 2.0 * v;
        Rectangle {
            coord: coord.with(|coord| {
                coord.move_to(p0);
            }),
            size: dvec2(width, width),
            stroke_rgba,
            stroke_width,
            fill_rgba,
        }
    }
}

impl From<Square> for RegPolygon {
    fn from(value: Square) -> Self {
        RegPolygon::new(4, value.size * f64::sqrt(0.5)).with(|item| {
            item.coord = value.coord.with(|coord| {
                coord.rotate_local(PI / 4.);
            });
        })
    }
}

impl From<Square> for Polygon {
    fn from(value: Square) -> Self {
        Rectangle::from(value).into()
    }
}

impl From<Square> for VItem {
    fn from(value: Square) -> Self {
        Rectangle::from(value).into()
    }
}
