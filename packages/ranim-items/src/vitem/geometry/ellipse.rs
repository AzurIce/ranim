#[cfg(feature = "render")]
use ranim_render::extract::{Extract, ExtractContext};

use ranim_core::{
    color::{AlphaColor, Srgb},
    glam::{DVec2, DVec3},
    traits::{Aabb, Discard, FillColor, Opacity, StrokeColor, With},
};

use crate::vitem::{
    DEFAULT_STROKE_WIDTH, VItem,
    geometry::{Circle, EllipticArc},
};

/// An ellipse centered at the origin in the XY plane.
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Ellipse {
    /// Semi-axes in the x and y directions.
    pub radius: DVec2,
    /// Stroke rgba.
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width.
    pub stroke_width: f32,
    /// Fill rgba.
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Ellipse {
    /// Creates a new ellipse.
    pub fn new(radius: DVec2) -> Self {
        Self {
            radius,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
}

impl From<Circle> for Ellipse {
    fn from(value: Circle) -> Self {
        let Circle {
            radius,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        } = value;
        Self {
            radius: DVec2::splat(radius),
            stroke_rgba,
            stroke_width,
            fill_rgba,
        }
    }
}

impl From<Ellipse> for VItem {
    fn from(value: Ellipse) -> Self {
        let fill_rgba = value.fill_rgba;
        VItem::from(EllipticArc::from(value)).with(|item| item.set_fill_color(fill_rgba).discard())
    }
}

impl Aabb for Ellipse {
    fn aabb(&self) -> [DVec3; 2] {
        let r = self.radius.extend(0.0);
        [-r, r].aabb()
    }
}

impl StrokeColor for Ellipse {
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

impl FillColor for Ellipse {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.fill_rgba
    }

    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self
    }

    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.fill_rgba = color;
        self
    }
}

impl Opacity for Ellipse {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.set_fill_opacity(opacity).set_stroke_opacity(opacity);
        self
    }
}

#[cfg(feature = "render")]
impl Extract for Ellipse {
    fn extract(&self, ctx: &mut ExtractContext<'_>) {
        VItem::from(self.clone()).extract(ctx);
    }
}
