use std::f64::consts::TAU;

use ranim_core::{
    Extract,
    color::{AlphaColor, Srgb},
    components::vpoint::VPointVec,
    core_item::{
        CoreItem,
        vitem::{Basis2d, DEFAULT_STROKE_WIDTH},
    },
    glam::{DVec2, DVec3},
    traits::{Aabb, Discard, Rotate, Shift, StrokeColor, StrokeWidth as _, With as _},
};

use crate::vitem::{
    VItem,
    geometry::{Arc, Circle, Ellipse},
};

/// An elliptic arc.
#[derive(Debug, Clone, ranim_macros::Interpolatable)]
pub struct EllipticArc {
    /// Basis
    pub basis: Basis2d,
    /// Center
    pub center: DVec3,
    /// Semi-axes in the x and y directions
    pub radius: DVec2,
    /// Start angle (measured by the theta parameter in parametric equation of the ellipse) in radians
    pub start_angle: f64,
    /// Span angle in radians
    pub angle: f64,
    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
}

impl EllipticArc {
    /// Creates a new elliptic arc.
    pub fn new(start_angle: f64, angle: f64, radius: DVec2) -> Self {
        EllipticArc {
            basis: Basis2d::default(),
            center: DVec3::ZERO,
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

        let &EllipticArc {
            basis,
            center,
            radius,
            start_angle,
            angle,
            ..
        } = self;

        let (u, v) = basis.uv();
        let DVec2 { x: rx, y: ry } = radius;
        let mut vpoints = (0..len)
            .map(|i| i as f64 / NUM_SEGMENTS as f64 / 2. * angle + start_angle)
            .map(|theta| {
                let (mut x, mut y) = (theta.cos(), theta.sin());
                if x.abs() < 1.8e-7 {
                    x = 0.;
                }
                if y.abs() < 1.8e-7 {
                    y = 0.;
                }
                x * rx * u + y * ry * v
            })
            .collect::<Vec<_>>();

        let k = (angle / NUM_SEGMENTS as f64 / 2.).cos();
        vpoints.iter_mut().skip(1).step_by(2).for_each(|p| *p /= k);
        vpoints.shift(center);
        vpoints
    }
}

impl From<Arc> for EllipticArc {
    fn from(value: Arc) -> Self {
        let Arc {
            basis,
            center,
            radius,
            angle,
            stroke_rgba,
            stroke_width,
        } = value;
        EllipticArc {
            basis,
            center,
            radius: DVec2::splat(radius),
            start_angle: 0.,
            angle,
            stroke_rgba,
            stroke_width,
        }
    }
}

impl From<Circle> for EllipticArc {
    fn from(value: Circle) -> Self {
        let Circle {
            basis,
            center,
            radius,
            stroke_rgba,
            stroke_width,
            ..
        } = value;
        EllipticArc {
            basis,
            center,
            radius: DVec2::splat(radius),
            start_angle: 0.,
            angle: TAU,
            stroke_rgba,
            stroke_width,
        }
    }
}

impl From<EllipticArc> for VItem {
    fn from(value: EllipticArc) -> Self {
        let EllipticArc {
            stroke_rgba,
            stroke_width,
            ..
        } = value;
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
            basis,
            center,
            radius,
            stroke_rgba,
            stroke_width,
            ..
        } = value;
        EllipticArc {
            basis,
            center,
            radius,
            start_angle: 0.,
            angle: TAU,
            stroke_rgba,
            stroke_width,
        }
    }
}

impl Aabb for EllipticArc {
    fn aabb(&self) -> [DVec3; 2] {
        // TODO: maybe calculate AABB by linear algebra?
        // that would be extremely complicated
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

impl Shift for EllipticArc {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.center += shift;
        self
    }
}

impl Rotate for EllipticArc {
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self {
        self.basis.rotate_axis(axis, angle);
        self.center.rotate_at_point(angle, axis, point);
        self
    }
}
