use std::f64::consts::PI;

use color::{AlphaColor, Srgb};
use glam::DVec3;
use ranim_core::{
    Extract,
    anchor::Aabb,
    color,
    core_item::CoreItem,
    glam,
    proj::CoordinateSystem,
    traits::{
        LocalCoordinate, Origin, Rotate, ScaleUniform, ScaleUniformByOrigin, ScaleUniformLocal,
        Shift,
    },
};

use crate::vitem::DEFAULT_STROKE_WIDTH;
use ranim_core::traits::{FillColor, Opacity, StrokeColor, With};

use crate::vitem::VItem;

use super::Arc;

// MARK: ### Circle ###
/// An circle
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Circle {
    /// Local coordinate system
    pub coord: CoordinateSystem,
    /// Radius
    pub radius: f64,

    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
    /// Fill rgba
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Circle {
    /// Constructor
    pub fn new(radius: f64) -> Self {
        Self {
            coord: CoordinateSystem::default(),
            radius,

            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
}

impl Origin for Circle {
    fn origin(&self) -> DVec3 {
        self.coord.origin()
    }
    fn move_to(&mut self, origin: DVec3) -> &mut Self {
        self.coord.move_to(origin);
        self
    }
}

// MARK: Traits impl
impl LocalCoordinate for Circle {
    fn coord(&self) -> ranim_core::proj::CoordinateSystem {
        self.coord
    }
}

impl ScaleUniformByOrigin for Circle {
    fn scale_uniform(&mut self, scale: f64) -> &mut Self {
        self.radius *= scale;
        self
    }
}

impl ScaleUniform for Circle {
    fn scale_uniform_at_point(&mut self, scale: f64, point: DVec3) -> &mut Self {
        self.coord.origin.scale_uniform_at_point(scale, point);
        self.radius *= scale;
        self
    }
}

impl ScaleUniformLocal for Circle {}

impl Aabb for Circle {
    fn aabb(&self) -> [DVec3; 2] {
        let center = self.coord().origin();
        let (u, v) = self.coord().basis();
        let r = self.radius * (u + v);
        [center + r, center - r].aabb()
    }
}

impl Shift for Circle {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.coord.shift(shift);
        self
    }
}

impl Rotate for Circle {
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self {
        self.coord.rotate_at_point(angle, axis, point);
        self
    }
}

impl Opacity for Circle {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgba = self.stroke_rgba.with_alpha(opacity);
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self
    }
}

impl StrokeColor for Circle {
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

impl FillColor for Circle {
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

// MARK: Conversions
impl From<Circle> for Arc {
    fn from(value: Circle) -> Self {
        let Circle {
            coord,
            radius,
            stroke_rgba,
            stroke_width,
            ..
        } = value;
        Self {
            coord,
            radius,
            angle: 2.0 * PI,
            stroke_rgba,
            stroke_width,
        }
    }
}

impl From<Circle> for VItem {
    fn from(value: Circle) -> Self {
        let fill_rgba = value.fill_rgba;
        VItem::from(Arc::from(value)).with(|item| {
            item.set_fill_color(fill_rgba);
        })
    }
}

impl Extract for Circle {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        VItem::from(self.clone()).extract_into(buf);
    }
}
