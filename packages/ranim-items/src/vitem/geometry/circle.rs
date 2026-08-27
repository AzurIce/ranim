use std::f64::consts::PI;

use color::{AlphaColor, Srgb};
use glam::DVec3;
use ranim_core::{
    Extract,
    anchor::Aabb,
    color,
    core_item::CoreItem,
    glam,
    traits::{FillColor, Opacity, StrokeColor, With},
};

use crate::vitem::{DEFAULT_STROKE_WIDTH, VItem};

use super::Arc;

/// A circle centered at the origin in the XY plane.
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Circle {
    /// Radius.
    pub radius: f64,
    /// Stroke rgba.
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width.
    pub stroke_width: f32,
    /// Fill rgba.
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Circle {
    /// Creates a circle with the given radius.
    pub fn new(radius: f64) -> Self {
        Self {
            radius,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }

    /// Scales the intrinsic radius.
    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.radius *= scale;
        self
    }
}

impl Aabb for Circle {
    fn aabb(&self) -> [DVec3; 2] {
        let r = DVec3::new(self.radius, self.radius, 0.0);
        [-r, r].aabb()
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

impl From<Circle> for Arc {
    fn from(value: Circle) -> Self {
        let Circle {
            radius,
            stroke_rgba,
            stroke_width,
            ..
        } = value;
        Self {
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
