use std::f64::consts::TAU;

use ranim_core::{
    Extract,
    color::{AlphaColor, Srgb},
    components::vpoint::VPointVec,
    core_item::{CoreItem, vitem::DEFAULT_STROKE_WIDTH},
    glam::{DVec2, DVec3},
    traits::{Aabb, Discard, Opacity, StrokeColor, StrokeWidth, With},
};

use crate::vitem::{
    VItem,
    geometry::{Arc, Circle, Ellipse},
};

/// An elliptic arc centered at the origin in the XY plane.
#[derive(Debug, Clone, ranim_macros::Interpolatable)]
pub struct EllipticArc {
    /// Semi-axes in the x and y directions.
    pub radius: DVec2,
    /// Start angle in radians.
    pub start_angle: f64,
    /// Span angle in radians.
    pub angle: f64,
    /// Stroke rgba.
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width.
    pub stroke_width: f32,
}

impl EllipticArc {
    /// Creates a new elliptic arc.
    pub fn new(start_angle: f64, angle: f64, radius: DVec2) -> Self {
        Self {
            radius,
            start_angle,
            angle,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
        }
    }

    fn generate_vpoints(&self) -> Vec<DVec3> {
        const NUM_SEGMENTS: usize = 8;
        let len = 2 * NUM_SEGMENTS + 1;
        let DVec2 { x: rx, y: ry } = self.radius;
        let mut vpoints = (0..len)
            .map(|i| i as f64 / NUM_SEGMENTS as f64 / 2.0 * self.angle + self.start_angle)
            .map(|theta| {
                let (mut x, mut y) = (theta.cos(), theta.sin());
                if x.abs() < 1.8e-7 {
                    x = 0.0;
                }
                if y.abs() < 1.8e-7 {
                    y = 0.0;
                }
                DVec3::new(x * rx, y * ry, 0.0)
            })
            .collect::<Vec<_>>();

        let k = (self.angle / NUM_SEGMENTS as f64 / 2.0).cos();
        vpoints.iter_mut().skip(1).step_by(2).for_each(|p| *p /= k);
        vpoints
    }
}

impl From<Arc> for EllipticArc {
    fn from(value: Arc) -> Self {
        let Arc {
            radius,
            angle,
            stroke_rgba,
            stroke_width,
        } = value;
        Self {
            radius: DVec2::splat(radius),
            start_angle: 0.0,
            angle,
            stroke_rgba,
            stroke_width,
        }
    }
}

impl From<Circle> for EllipticArc {
    fn from(value: Circle) -> Self {
        let Circle {
            radius,
            stroke_rgba,
            stroke_width,
            ..
        } = value;
        Self {
            radius: DVec2::splat(radius),
            start_angle: 0.0,
            angle: TAU,
            stroke_rgba,
            stroke_width,
        }
    }
}

impl From<EllipticArc> for VItem {
    fn from(value: EllipticArc) -> Self {
        let stroke_rgba = value.stroke_rgba;
        let stroke_width = value.stroke_width;
        VItem::from_vpoints(value.generate_vpoints()).with(|vitem| {
            vitem
                .set_stroke_color(stroke_rgba)
                .set_stroke_width(stroke_width)
                .discard()
        })
    }
}

impl From<Ellipse> for EllipticArc {
    fn from(value: Ellipse) -> Self {
        let Ellipse {
            radius,
            stroke_rgba,
            stroke_width,
            ..
        } = value;
        Self {
            radius,
            start_angle: 0.0,
            angle: TAU,
            stroke_rgba,
            stroke_width,
        }
    }
}

impl Aabb for EllipticArc {
    fn aabb(&self) -> [DVec3; 2] {
        VPointVec(self.generate_vpoints()).aabb()
    }
}

impl Extract for EllipticArc {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        VItem::from(self.clone()).extract_into(buf);
    }
}

impl StrokeColor for EllipticArc {
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

impl Opacity for EllipticArc {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.set_stroke_opacity(opacity);
        self
    }
}
