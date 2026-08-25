//! Transformed — a wrapper that attaches an external affine transform to an item.

use glam::{DAffine3, DVec3, dvec3};

use crate::{
    Extract,
    anchor::Aabb,
    color::{AlphaColor, Srgb},
    core_item::CoreItem,
    traits::{
        Alignable, ApplyTransform, FillColor, Interpolatable, NotSimilarity, Opacity, Similarity,
        StrokeColor,
    },
};

/// A wrapper that keeps `inner` in local coordinates and stores an affine
/// transform from those coordinates to the wrapper's parent coordinates.
/// For an outermost wrapper, the parent coordinate system is normally world
/// space.
///
/// Applying transforms to a `Transformed<T>` never touches `inner`'s data —
/// it only composes the stored transform (lazy "boxing"):
///
/// - [`ApplyTransform::apply`] / `shift` / `rotate` / `scale` — action in the
///   parent coordinate system, using left-multiplication:
///   `transform = G * transform`.
/// - [`Transformed::apply_local`] — action in `inner`'s local coordinate
///   system, using right-multiplication: `transform = transform * G`.
///
/// Both accept any affine `G` because the wrapper preserves `inner`'s
/// semantic representation. Constraints appear only when folding the stored
/// transform back into that representation:
///
/// - [`Transformed::bake`] — unconditional, for affine-closure `T`.
/// - [`Transformed::try_bake`] — runtime-checked, for similarity-closure
///   `T` (the stored transform must actually be a similarity).
///
/// Nesting composes from the inside out:
/// `Transformed { t1, inner: Transformed { t2, inner: x } }` maps `x` into
/// the outer wrapper's parent coordinates with `t1 * t2`. Extraction folds
/// the composed transform into the emitted [`CoreItem`]s (see
/// [`CoreItem::apply_transform`]).
///
/// Interpolation lerps `transform` and `inner` **independently** — this is
/// the intended behavior. Note that for e.g. perspective-blend scenes,
/// wrapping in `Transformed` may make points at the same world position on
/// different faces interpolate along different paths; if a raw point-data
/// morph is wanted, bake the transform into the underlying data (e.g.
/// `VItem` points) and interpolate that instead.
///
/// The transform is stored as [`DAffine3`], so it is affine **by
/// construction** — projective matrices are unrepresentable here.
/// Perspective lives solely in the camera projection, never in model
/// transforms.
#[derive(Debug, Clone, PartialEq)]
pub struct Transformed<T> {
    /// The affine transform from `inner`'s local coordinates to parent coordinates.
    pub transform: DAffine3,
    /// The item expressed in its own local coordinates.
    pub inner: T,
}

impl<T> Transformed<T> {
    /// Wrap `inner` with an identity transform.
    pub fn new(inner: T) -> Self {
        Self {
            transform: DAffine3::IDENTITY,
            inner,
        }
    }

    /// Set the transform. Returns `self` for chaining.
    pub fn with_transform(mut self, transform: DAffine3) -> Self {
        self.transform = transform;
        self
    }

    /// Apply a transform in `inner`'s local coordinate system
    /// (right-multiplication: `transform = transform * G`).
    ///
    /// This is the generic way to transform an item along its own axes, such
    /// as scaling a rotated rectangle along its edges.
    pub fn apply_local<G: Into<DAffine3>>(&mut self, transform: G) -> &mut Self {
        self.transform *= transform.into();
        self
    }

    /// Fold the stored transform into `inner`'s data and unwrap.
    ///
    /// Only available for affine-closure `T` (point-data types). For
    /// similarity-closure types use [`Transformed::try_bake`], or
    /// [`Extract`] the wrapper to bake into [`CoreItem`] point data instead.
    pub fn bake(self) -> T
    where
        T: ApplyTransform<DAffine3>,
    {
        let mut inner = self.inner;
        inner.apply(self.transform);
        inner
    }

    /// Fold the stored transform into `inner`'s data and unwrap, checking at
    /// runtime that it is actually a similarity transform.
    ///
    /// Available for similarity-closure `T` (circles, spheres, ...). Fails
    /// with [`NotSimilarity`] if the accumulated transform contains
    /// non-uniform scale, shear, or reflection.
    pub fn try_bake(self) -> Result<T, NotSimilarity>
    where
        T: ApplyTransform<Similarity>,
    {
        let similarity = Similarity::try_from(self.transform)?;
        let mut inner = self.inner;
        inner.apply(similarity);
        Ok(inner)
    }
}

impl<G: Into<DAffine3>, T> ApplyTransform<G> for Transformed<T> {
    /// Action in the parent coordinate system
    /// (left-multiplication: `transform = G * transform`).
    ///
    /// Never bakes — the stored transform absorbs any affine transform and
    /// `inner`'s data (and thus its semantics) is left untouched.
    fn apply(&mut self, transform: G) -> &mut Self {
        self.transform = transform.into() * self.transform;
        self
    }
}

impl<T: Extract<Target = CoreItem>> Extract for Transformed<T> {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        let start = buf.len();
        self.inner.extract_into(buf);
        for item in &mut buf[start..] {
            item.apply_transform(&self.transform);
        }
    }
}

impl<T: Interpolatable> Interpolatable for Transformed<T> {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            transform: self.transform.lerp(&target.transform, t),
            inner: self.inner.lerp(&target.inner, t),
        }
    }
}

impl<T: Alignable> Alignable for Transformed<T> {
    fn is_aligned(&self, other: &Self) -> bool {
        self.inner.is_aligned(&other.inner)
    }

    fn align_with(&mut self, other: &mut Self) {
        self.inner.align_with(&mut other.inner);
    }
}

impl<T: Aabb> Aabb for Transformed<T> {
    fn aabb(&self) -> [DVec3; 2] {
        let [min, max] = self.inner.aabb();
        // Transform the 8 corners of the inner AABB and re-bound.
        let mut lo = DVec3::splat(f64::INFINITY);
        let mut hi = DVec3::splat(f64::NEG_INFINITY);
        for i in 0..8 {
            let corner = dvec3(
                if i & 1 == 0 { min.x } else { max.x },
                if i & 2 == 0 { min.y } else { max.y },
                if i & 4 == 0 { min.z } else { max.z },
            );
            let p = self.transform.transform_point3(corner);
            lo = lo.min(p);
            hi = hi.max(p);
        }
        [lo, hi]
    }
}

impl<T: Opacity> Opacity for Transformed<T> {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.inner.set_opacity(opacity);
        self
    }
}

impl<T: FillColor> FillColor for Transformed<T> {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.inner.fill_color()
    }

    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.inner.set_fill_color(color);
        self
    }

    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.inner.set_fill_opacity(opacity);
        self
    }
}

impl<T: StrokeColor> StrokeColor for Transformed<T> {
    fn stroke_color(&self) -> AlphaColor<Srgb> {
        self.inner.stroke_color()
    }

    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.inner.set_stroke_color(color);
        self
    }

    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.inner.set_stroke_opacity(opacity);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        core_item::{mesh_item::MeshItem, vitem::VItem},
        traits::{Diag, Rigid, ScaleTransform, ShiftTransform},
    };
    use glam::{DQuat, DVec3, Vec3, Vec4, dvec3};

    #[test]
    fn test_extract_mesh_item_composes_transform() {
        let mesh = MeshItem {
            transform: glam::Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0)),
            ..Default::default()
        };
        let wrapped =
            Transformed::new(mesh).with_transform(DAffine3::from_translation(dvec3(0.0, 2.0, 0.0)));
        let items = wrapped.extract();
        assert_eq!(items.len(), 1);
        match &items[0] {
            CoreItem::MeshItem(m) => {
                assert_eq!(
                    m.transform,
                    glam::Mat4::from_translation(Vec3::new(1.0, 2.0, 0.0))
                );
                // Vertices untouched
                assert_eq!(m.points, vec![Vec3::ZERO; 3]);
            }
            _ => panic!("expected MeshItem"),
        }
    }

    #[test]
    fn test_extract_vitem_bakes_points() {
        let vitem = VItem {
            points: vec![
                Vec4::new(1.0, 0.0, 0.0, 0.0),
                Vec4::new(0.0, 1.0, 0.0, 0.0),
                Vec4::new(0.0, 0.0, 1.0, 0.0),
            ],
            ..Default::default()
        };
        let wrapped = Transformed::new(vitem)
            .with_transform(DAffine3::from_translation(dvec3(0.0, 2.0, 0.0)));
        let items = wrapped.extract();
        match &items[0] {
            CoreItem::VItem(v) => {
                assert_eq!(v.points[0], Vec4::new(1.0, 2.0, 0.0, 0.0));
                assert_eq!(v.points[1], Vec4::new(0.0, 3.0, 0.0, 0.0));
                assert_eq!(v.points[2], Vec4::new(0.0, 2.0, 1.0, 0.0));
            }
            _ => panic!("expected VItem"),
        }
    }

    #[test]
    fn test_extract_vitem_uses_inverse_transpose_for_normal() {
        let vitem = VItem {
            normal: Some(Vec3::new(1.0, 1.0, 0.0).normalize()),
            ..Default::default()
        };
        let wrapped =
            Transformed::new(vitem).with_transform(DAffine3::from_scale(dvec3(2.0, 1.0, 1.0)));
        let items = wrapped.extract();
        match &items[0] {
            CoreItem::VItem(v) => {
                let expected = Vec3::new(0.5, 1.0, 0.0).normalize();
                assert!(v.normal.unwrap().abs_diff_eq(expected, 1e-6));
            }
            _ => panic!("expected VItem"),
        }
    }

    #[test]
    fn test_nested_transform_composes() {
        let inner = Transformed::new(VItem {
            points: vec![Vec4::new(1.0, 0.0, 0.0, 0.0)],
            ..Default::default()
        })
        .with_transform(DAffine3::from_translation(dvec3(1.0, 0.0, 0.0)));
        let outer = Transformed::new(inner).with_transform(DAffine3::from_scale(DVec3::splat(2.0)));
        let items = outer.extract();
        match &items[0] {
            CoreItem::VItem(v) => {
                assert_eq!(v.points[0], Vec4::new(4.0, 0.0, 0.0, 0.0));
            }
            _ => panic!("expected VItem"),
        }
    }

    #[test]
    fn test_operation_traits_compose_in_parent_coordinates() {
        // shift/rotate/scale are blanket-derived from ApplyTransform and act
        // in the parent coordinate system (left-multiplication).
        let mut wrapped = Transformed::new(());
        wrapped.shift(dvec3(1.0, 0.0, 0.0));
        wrapped.scale(DVec3::splat(2.0));
        assert_eq!(
            wrapped.transform,
            DAffine3::from_scale(DVec3::splat(2.0))
                * DAffine3::from_translation(dvec3(1.0, 0.0, 0.0))
        );
    }

    #[test]
    fn test_apply_local_composes_on_the_right() {
        let mut wrapped = Transformed::new(());
        wrapped.shift(dvec3(1.0, 0.0, 0.0));
        wrapped.apply_local(Diag(DVec3::splat(2.0)));
        assert_eq!(
            wrapped.transform,
            DAffine3::from_translation(dvec3(1.0, 0.0, 0.0))
                * DAffine3::from_scale(DVec3::splat(2.0))
        );
    }

    #[test]
    fn test_apply_accepts_group_typed_values() {
        let mut wrapped = Transformed::new(());
        wrapped.apply(Rigid {
            rotation: DQuat::from_axis_angle(DVec3::Z, core::f64::consts::FRAC_PI_2),
            translation: dvec3(1.0, 0.0, 0.0),
        });
        let p = wrapped.transform.transform_point3(dvec3(1.0, 0.0, 0.0));
        assert!(p.abs_diff_eq(dvec3(1.0, 1.0, 0.0), 1e-9));
    }

    #[test]
    fn test_bake_explicitly_folds_affine_frame_into_inner() {
        let wrapped = Transformed::new(dvec3(1.0, 0.0, 0.0))
            .with_transform(DAffine3::from_translation(dvec3(0.0, 2.0, 0.0)));
        assert_eq!(wrapped.bake(), dvec3(1.0, 2.0, 0.0));
    }

    #[test]
    fn test_lerp_transform_and_inner_independently() {
        let a = Transformed::new(0.0f64)
            .with_transform(DAffine3::from_translation(dvec3(0.0, 0.0, 0.0)));
        let b = Transformed::new(2.0f64)
            .with_transform(DAffine3::from_translation(dvec3(2.0, 0.0, 0.0)));
        let mid = a.lerp(&b, 0.5);
        assert_eq!(mid.inner, 1.0);
        assert_eq!(
            mid.transform,
            DAffine3::from_translation(dvec3(1.0, 0.0, 0.0))
        );
    }

    #[test]
    fn test_aabb_transforms_corners() {
        let wrapped = Transformed::new(dvec3(1.0, 1.0, 1.0))
            .with_transform(DAffine3::from_translation(dvec3(10.0, 0.0, 0.0)));
        let [min, max] = wrapped.aabb();
        assert_eq!(min, dvec3(11.0, 1.0, 1.0));
        assert_eq!(max, dvec3(11.0, 1.0, 1.0));
    }
}
