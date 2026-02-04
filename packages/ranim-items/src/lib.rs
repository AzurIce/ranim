//! Ranim's built-in items
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(rustdoc::private_intra_doc_links)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg",
    html_favicon_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg"
)]

use ranim_core::{
    glam::DVec3,
    traits::{Aabb, AnchorPoint, Interpolatable, Rotate, Shift},
};

pub mod vitem;

/// A plane in 3D space.
#[derive(Debug, Clone, PartialEq)]
pub struct Plane {
    /// The origin of the plane.
    pub origin: DVec3,
    /// The basis vectors of the plane. Normalized.
    pub basis: (DVec3, DVec3),
}

impl Interpolatable for Plane {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            origin: self.origin.lerp(target.origin, t),
            basis: (
                self.basis.0.lerp(target.basis.0, t),
                self.basis.1.lerp(target.basis.1, t),
            ),
        }
    }
}

impl Aabb for Plane {
    fn aabb(&self) -> [DVec3; 2] {
        let basis_vec = self.basis.0 + self.basis.1;
        [self.origin - basis_vec / 2.0, self.origin + basis_vec / 2.0]
    }
}

impl Shift for Plane {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.origin.shift(shift);
        self
    }
}

impl Rotate for Plane {
    fn rotate_at(&mut self, angle: f64, axis: DVec3, anchor_point: impl AnchorPoint) -> &mut Self {
        self.origin.rotate_at(angle, axis, anchor_point);
        self.basis.0.rotate(angle, axis);
        self.basis.1.rotate(angle, axis);
        self
    }
}

impl Default for Plane {
    fn default() -> Self {
        Self::XY
    }
}

impl Plane {
    const XY: Self = Self {
        origin: DVec3::ZERO,
        basis: (DVec3::X, DVec3::Y),
    };
}
