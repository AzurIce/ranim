//! Transformation group element types.
//!
//! These types model the transformation-group hierarchy that ranim supports.
//! Model transforms stop at the affine group — projective transforms live
//! solely in the camera projection:
//!
//! ```text
//! Translation ──> Rigid ──> Similarity ──> DAffine3 (Aff(3))
//!    (T(3))      (SE(3))     (Sim(3))          ▲
//!                     Diag (axis-aligned scaling) ──┘
//! ```
//!
//! Group containment is encoded as **lossless [`From`] conversions**
//! (embeddings up the hierarchy); the fallible downward direction is
//! [`TryFrom`] (e.g. [`DAffine3`] → [`Similarity`] requires the linear part
//! to be a uniform-scale rotation).
//!
//! Shapes declare their *closure group* by implementing
//! [`ApplyTransform`](super::ApplyTransform) with a `G: Into<...>` bound
//! (e.g. `G: Into<DAffine3>` for affine-closure point data, or
//! `G: Into<Similarity>` for circles and spheres); the operation traits
//! ([`ShiftTransform`](crate::traits::ShiftTransform),
//! [`RotateTransform`](crate::traits::RotateTransform),
//! [`ScaleTransform`](crate::traits::ScaleTransform),
//! [`UniformScaleTransform`](super::UniformScaleTransform)) are
//! blanket-derived from it.

use core::ops::{Deref, DerefMut};

use glam::{DAffine3, DMat3, DQuat, DVec3};

use crate::traits::Interpolatable;

// MARK: Translation
/// The translation group T(3): pure displacements.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Translation(pub DVec3);

impl From<DVec3> for Translation {
    fn from(v: DVec3) -> Self {
        Self(v)
    }
}

impl From<Translation> for DVec3 {
    fn from(t: Translation) -> Self {
        t.0
    }
}

impl Deref for Translation {
    type Target = DVec3;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Translation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Interpolatable for Translation {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self(self.0.lerp(target.0, t))
    }
}

// MARK: Rigid
/// The rigid (Euclidean) group SE(3): rotation + translation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rigid {
    /// The rotation part.
    pub rotation: DQuat,
    /// The translation part.
    pub translation: DVec3,
}

impl Rigid {
    /// The identity rigid transform.
    pub const IDENTITY: Self = Self {
        rotation: DQuat::IDENTITY,
        translation: DVec3::ZERO,
    };

    /// A pure rotation.
    pub fn from_rotation(rotation: DQuat) -> Self {
        Self {
            rotation,
            translation: DVec3::ZERO,
        }
    }

    /// A pure translation.
    pub fn from_translation(translation: DVec3) -> Self {
        Self {
            rotation: DQuat::IDENTITY,
            translation,
        }
    }

    /// A pure rotation of `angle` radians about `axis` (normalized internally).
    pub fn from_axis_angle(axis: DVec3, angle: f64) -> Self {
        Self::from_rotation(DQuat::from_axis_angle(axis.normalize(), angle))
    }
}

impl Interpolatable for Rigid {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            rotation: self.rotation.slerp(target.rotation, t),
            translation: self.translation.lerp(target.translation, t),
        }
    }
}

// MARK: Similarity
/// The similarity group Sim(3): uniform scale + rotation + translation.
///
/// This is the closure group of shapes defined by angles and length ratios
/// (circles, spheres, squares, regular polygons, circular arcs): baking any
/// similarity into them preserves their semantics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Similarity {
    /// The finite, strictly positive uniform scale factor.
    pub scale: f64,
    /// The rotation part.
    pub rotation: DQuat,
    /// The translation part.
    pub translation: DVec3,
}

impl Similarity {
    /// The identity similarity.
    pub const IDENTITY: Self = Self {
        scale: 1.0,
        rotation: DQuat::IDENTITY,
        translation: DVec3::ZERO,
    };

    /// A pure uniform scale.
    ///
    /// # Panics
    ///
    /// Panics if `scale` is not finite and strictly positive, because zero and
    /// orientation-reversing scales are not elements of `Sim(3)` as modeled
    /// here.
    pub fn from_scale(scale: f64) -> Self {
        assert!(
            scale.is_finite() && scale > 0.0,
            "similarity scale must be finite and strictly positive"
        );
        Self {
            scale,
            rotation: DQuat::IDENTITY,
            translation: DVec3::ZERO,
        }
    }

    /// Transform a point: `s * (R * p) + t`.
    pub fn transform_point(&self, p: DVec3) -> DVec3 {
        self.scale * (self.rotation * p) + self.translation
    }

    /// Transform a direction vector (rotation only, preserving unit length).
    pub fn transform_direction(&self, v: DVec3) -> DVec3 {
        self.rotation * v
    }
}

impl Interpolatable for Similarity {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            scale: self.scale.lerp(&target.scale, t),
            rotation: self.rotation.slerp(target.rotation, t),
            translation: self.translation.lerp(target.translation, t),
        }
    }
}

// MARK: Diag
/// The diagonal group (R\*)³: axis-aligned non-uniform scaling.
///
/// Note that `Diag` does **not** compose with rotations in a closed way
/// (`R₁ · Diag · R₂` generally introduces shear), which is why it is not a
/// subgroup of [`Similarity`] — only of the affine group.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Diag(pub DVec3);

impl From<DVec3> for Diag {
    fn from(v: DVec3) -> Self {
        Self(v)
    }
}

impl From<Diag> for DVec3 {
    fn from(d: Diag) -> Self {
        d.0
    }
}

impl Deref for Diag {
    type Target = DVec3;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Diag {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Interpolatable for Diag {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self(self.0.lerp(target.0, t))
    }
}

// MARK: Embeddings
// Lossless conversions up the group hierarchy (group containment).
// Note: `From` is not transitive in Rust, so every edge is written out.

impl From<Translation> for Rigid {
    fn from(t: Translation) -> Self {
        Rigid::from_translation(t.0)
    }
}

impl From<Translation> for Similarity {
    fn from(t: Translation) -> Self {
        Similarity {
            scale: 1.0,
            rotation: DQuat::IDENTITY,
            translation: t.0,
        }
    }
}

impl From<Translation> for DAffine3 {
    fn from(t: Translation) -> Self {
        DAffine3::from_translation(t.0)
    }
}

impl From<Rigid> for Similarity {
    fn from(r: Rigid) -> Self {
        Similarity {
            scale: 1.0,
            rotation: r.rotation,
            translation: r.translation,
        }
    }
}

impl From<Rigid> for DAffine3 {
    fn from(r: Rigid) -> Self {
        DAffine3::from_rotation_translation(r.rotation, r.translation)
    }
}

impl From<Similarity> for DAffine3 {
    fn from(s: Similarity) -> Self {
        DAffine3::from_scale_rotation_translation(DVec3::splat(s.scale), s.rotation, s.translation)
    }
}

impl From<Diag> for DAffine3 {
    fn from(d: Diag) -> Self {
        DAffine3::from_scale(d.0)
    }
}

// MARK: NotSimilarity
/// Error returned when an affine transform is not a similarity transform
/// (its linear part has non-uniform scale, shear, or reflection).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotSimilarity;

impl core::fmt::Display for NotSimilarity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("the affine transform is not a similarity transform")
    }
}

impl std::error::Error for NotSimilarity {}

impl TryFrom<DAffine3> for Similarity {
    type Error = NotSimilarity;

    /// Extract a similarity from an affine transform, checking that the
    /// linear part is a uniform-scale rotation (three equal singular
    /// values, no shear, no reflection).
    fn try_from(affine: DAffine3) -> Result<Self, Self::Error> {
        const EPS: f64 = 1e-9;

        let m = affine.matrix3;
        if !m.is_finite() || !affine.translation.is_finite() {
            return Err(NotSimilarity);
        }
        let cols = [m.x_axis, m.y_axis, m.z_axis];
        let s = cols[0].length();
        if s <= EPS {
            return Err(NotSimilarity);
        }
        // Equal column lengths (uniform scale) ...
        if (cols[1].length() - s).abs() > EPS * s || (cols[2].length() - s).abs() > EPS * s {
            return Err(NotSimilarity);
        }
        // ... mutual orthogonality (no shear) ...
        if cols[0].dot(cols[1]).abs() > EPS * s * s
            || cols[0].dot(cols[2]).abs() > EPS * s * s
            || cols[1].dot(cols[2]).abs() > EPS * s * s
        {
            return Err(NotSimilarity);
        }
        // ... and positive orientation (no reflection).
        if m.determinant() <= 0.0 {
            return Err(NotSimilarity);
        }
        let rotation = DQuat::from_mat3(&DMat3::from_cols(cols[0] / s, cols[1] / s, cols[2] / s));
        Ok(Self {
            scale: s,
            rotation,
            translation: affine.translation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::dvec3;

    #[test]
    fn test_embeddings_compose_to_same_affine() {
        // Translation -> Rigid -> Similarity -> DAffine3 must agree with
        // Translation -> DAffine3 directly.
        let t = Translation(dvec3(1.0, 2.0, 3.0));
        let direct = DAffine3::from(t);
        let via: DAffine3 = Similarity::from(Rigid::from(t)).into();
        assert_eq!(direct, via);
    }

    #[test]
    fn test_similarity_try_from_accepts_similarity() {
        let sim = Similarity {
            scale: 2.5,
            rotation: DQuat::from_axis_angle(DVec3::Z, 0.7),
            translation: dvec3(1.0, 2.0, 3.0),
        };
        let affine = DAffine3::from(sim);
        let back = Similarity::try_from(affine).unwrap();
        assert!((back.scale - 2.5).abs() < 1e-9);
        assert!(back.translation.abs_diff_eq(sim.translation, 1e-9));
        // Rotation: q and -q represent the same rotation
        let rotated = back.rotation * DVec3::X;
        let expected = sim.rotation * DVec3::X;
        assert!(rotated.abs_diff_eq(expected, 1e-9));
    }

    #[test]
    fn test_similarity_try_from_rejects_non_uniform_scale() {
        let affine = DAffine3::from_scale(dvec3(1.0, 2.0, 1.0));
        assert!(Similarity::try_from(affine).is_err());
    }

    #[test]
    fn test_similarity_try_from_rejects_reflection() {
        let affine = DAffine3::from_scale(dvec3(-1.0, 1.0, 1.0));
        assert!(Similarity::try_from(affine).is_err());
    }

    #[test]
    fn test_similarity_try_from_rejects_non_finite_values() {
        let affine = DAffine3::from_scale(DVec3::splat(f64::NAN));
        assert!(Similarity::try_from(affine).is_err());
    }

    #[test]
    #[should_panic(expected = "similarity scale must be finite and strictly positive")]
    fn test_similarity_from_scale_rejects_non_positive_values() {
        Similarity::from_scale(0.0);
    }

    #[test]
    fn test_similarity_transform_point() {
        let sim = Similarity {
            scale: 2.0,
            rotation: DQuat::from_axis_angle(DVec3::Z, core::f64::consts::FRAC_PI_2),
            translation: dvec3(1.0, 0.0, 0.0),
        };
        let p = sim.transform_point(dvec3(1.0, 0.0, 0.0));
        assert!(p.abs_diff_eq(dvec3(1.0, 2.0, 0.0), 1e-9));
    }
}
