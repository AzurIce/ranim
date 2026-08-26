/// Types of transforms based on group theory.
pub mod group;
mod rotate;
mod scale;
mod shift;

pub use group::{Diag, NotSimilarity, Rigid, Similarity, TransformGroup, Translation};
pub use rotate::RotateTransform;
pub use scale::{
    ScaleHint, ScaleTransform, ScaleTransformExt, ScaleTransformStrokeExt, UniformScaleTransform,
};
pub use shift::{ShiftTransform, ShiftTransformExt};

use glam::{DAffine3, DVec3};

// MARK: ApplyTransform
/// A group action of `G` on `Self`: baking a transform into the item's data.
///
/// This is the single primitive trait of the transform system. A type
/// declares its *closure group* by the bound it implements this trait for:
///
/// - `impl<G: Into<DAffine3>> ApplyTransform<G> for T` — **affine closure**
///   (point-data types: `VItem`, polygons, meshes, ...),
/// - `impl<G: Into<Similarity>> ApplyTransform<G> for T` — **similarity
///   closure** (circles, spheres, squares, circular arcs, ...).
///
/// The operation traits ([`ShiftTransform`], [`RotateTransform`],
/// [`ScaleTransform`], [`UniformScaleTransform`]) are blanket-derived from
/// this trait, so implementing `ApplyTransform<G>` provides them
/// automatically — and opts out of implementing them manually (the
/// coherence conflict is intentional: it keeps one method name meaning one
/// group action across all types).
pub trait ApplyTransform<G> {
    /// Bake `transform` (expressed in world coordinates) into the item's
    /// data, in place.
    fn apply(&mut self, transform: G) -> &mut Self;
}

impl<G: Into<DAffine3>> ApplyTransform<G> for DVec3 {
    fn apply(&mut self, transform: G) -> &mut Self {
        *self = transform.into().transform_point3(*self);
        self
    }
}

impl<G: Copy + Into<DAffine3>, T: ApplyTransform<G>> ApplyTransform<G> for [T] {
    fn apply(&mut self, transform: G) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.apply(transform);
        });
        self
    }
}

impl<G: Copy + Into<DAffine3>, T: ApplyTransform<G>> ApplyTransform<G> for Vec<T> {
    fn apply(&mut self, transform: G) -> &mut Self {
        self.as_mut_slice().apply(transform);
        self
    }
}

// MARK: Blanket derivations
// One method name, one group action. The bounds name the required group
// precisely: shifting needs T(3), rotating needs SE(3), (axis-aligned)
// scaling needs Diag, uniform scaling needs Sim(3).

impl<T: ApplyTransform<Translation> + ?Sized> ShiftTransform for T {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.apply(Translation(offset))
    }
}

impl<T: ApplyTransform<Rigid> + ?Sized> RotateTransform for T {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.apply(Rigid::from_axis_angle(axis, angle))
    }
}

impl<T: ApplyTransform<Diag> + ?Sized> ScaleTransform for T {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.apply(Diag(scale))
    }
}
