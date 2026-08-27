//! A wrapper that attaches a typed external transform to an item.

use glam::{DAffine3, DVec3, dvec3};

use crate::{
    Extract,
    anchor::{Aabb, Locate},
    color::{AlphaColor, Srgb},
    core_item::CoreItem,
    traits::{
        Alignable, ApplyTransform, Diag, FillColor, Interpolatable, Opacity, Rigid, Similarity,
        StrokeColor, TransformGroup, Translation,
    },
};

/// An item paired with an external transform represented by `G`.
///
/// `inner` retains its own representation while `transform` controls the
/// extracted geometry. Composition is explicit about order:
///
/// - [`compose_outer`](Self::compose_outer): `transform = outer * transform`;
/// - [`compose_inner`](Self::compose_inner): `transform = transform * inner`.
///
/// Applying an `H` through [`ApplyTransform`] is outer composition and is only
/// available when `H` embeds into the existing storage type (`G: From<H>`).
/// Operations never widen the wrapper automatically. Widen explicitly with
/// [`Into::into`] when a more general storage type is required.
///
/// ```compile_fail
/// use ranim_core::{glam::DVec3, prelude::*};
///
/// let mut item = ().transformed(Similarity::IDENTITY);
/// // `Diag` does not embed into `Similarity`; choose `DAffine3` storage first.
/// item.scale(DVec3::new(2.0, 1.0, 1.0));
/// ```
///
/// Extraction and [`Aabb`] calculation convert `G` to [`DAffine3`] only at the
/// geometry boundary. Projective transforms remain outside the model-transform
/// system.
#[derive(Debug, Clone, PartialEq)]
pub struct Transformed<T, G> {
    /// The item expressed in its own coordinates.
    pub inner: T,
    /// The external transform applied to `inner`.
    pub transform: G,
}

impl<T, G> Transformed<T, G> {
    /// Pair `inner` with `transform` without converting either value.
    pub fn new(inner: T, transform: G) -> Self {
        Self { inner, transform }
    }

    /// Map the wrapped item to a new type, keeping `transform` unchanged.
    pub fn map_inner<U, F>(self, f: F) -> Transformed<U, G>
    where
        F: FnOnce(T) -> U,
    {
        Transformed::new(f(self.inner), self.transform)
    }

    /// Map the transform storage to a new type while keeping `inner`
    /// unchanged.
    ///
    /// This is the general form of converting between transform groups —
    /// for example widening a placement or checked-narrowing an affine
    /// storage back to a subgroup. Lossless upward conversions need no
    /// closure at all: [`Transformed`] implements
    /// `From<Transformed<T, X>> for Transformed<T, Y>` whenever `X` embeds
    /// into `Y`, so prefer plain `.into()` there.
    pub fn map_transform<H, F>(self, f: F) -> Transformed<T, H>
    where
        F: FnOnce(G) -> H,
    {
        Transformed::new(self.inner, f(self.transform))
    }

    /// Compose `outer` on the left: `transform = outer * transform`.
    pub fn compose_outer<H>(&mut self, outer: H) -> &mut Self
    where
        G: TransformGroup + From<H>,
    {
        self.transform = G::from(outer).compose(&self.transform);
        self
    }

    /// Compose `inner` on the right: `transform = transform * inner`.
    pub fn compose_inner<H>(&mut self, inner: H) -> &mut Self
    where
        G: TransformGroup + From<H>,
    {
        self.transform = self.transform.compose(&G::from(inner));
        self
    }

    /// Bake the stored transform into `inner` and remove the wrapper.
    ///
    /// This method exists only when `T` directly supports the wrapper's exact
    /// transform representation `G`.
    pub fn bake(self) -> T
    where
        T: ApplyTransform<G>,
    {
        let mut inner = self.inner;
        inner.apply(self.transform);
        inner
    }
}

impl<T, G, H> ApplyTransform<H> for Transformed<T, G>
where
    G: TransformGroup + From<H>,
{
    fn apply(&mut self, transform: H) -> &mut Self {
        self.compose_outer(transform)
    }
}

impl<T, G> Extract for Transformed<T, G>
where
    T: Extract<Target = CoreItem>,
    G: Clone + Into<DAffine3>,
{
    type Target = CoreItem;

    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        let start = buf.len();
        self.inner.extract_into(buf);
        let transform = self.transform.clone().into();
        for item in &mut buf[start..] {
            item.apply_transform(&transform);
        }
    }
}

impl<T: Interpolatable, G: Interpolatable> Interpolatable for Transformed<T, G> {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            inner: self.inner.lerp(&target.inner, t),
            transform: self.transform.lerp(&target.transform, t),
        }
    }
}

impl<T: Alignable, G: Clone> Alignable for Transformed<T, G> {
    fn is_aligned(&self, other: &Self) -> bool {
        self.inner.is_aligned(&other.inner)
    }

    fn align_with(&mut self, other: &mut Self) {
        self.inner.align_with(&mut other.inner);
    }
}

impl<T, G> Locate<Transformed<T, G>> for crate::anchor::Centroid
where
    crate::anchor::Centroid: Locate<T>,
    G: Clone + Into<DAffine3>,
{
    fn locate(&self, target: &Transformed<T, G>) -> DVec3 {
        target
            .transform
            .clone()
            .into()
            .transform_point3(self.locate(&target.inner))
    }
}

impl<T, G> Aabb for Transformed<T, G>
where
    T: Aabb,
    G: Clone + Into<DAffine3>,
{
    fn aabb(&self) -> [DVec3; 2] {
        let [min, max] = self.inner.aabb();
        let transform = self.transform.clone().into();
        let mut lo = DVec3::splat(f64::INFINITY);
        let mut hi = DVec3::splat(f64::NEG_INFINITY);
        for i in 0..8 {
            let corner = dvec3(
                if i & 1 == 0 { min.x } else { max.x },
                if i & 2 == 0 { min.y } else { max.y },
                if i & 4 == 0 { min.z } else { max.z },
            );
            let point = transform.transform_point3(corner);
            lo = lo.min(point);
            hi = hi.max(point);
        }
        [lo, hi]
    }
}

impl<T: Opacity, G> Opacity for Transformed<T, G> {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.inner.set_opacity(opacity);
        self
    }
}

impl<T: FillColor, G> FillColor for Transformed<T, G> {
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

impl<T: StrokeColor, G> StrokeColor for Transformed<T, G> {
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

macro_rules! impl_transformed_widening {
    ($source:ty => $target:ty) => {
        impl<T> From<Transformed<T, $source>> for Transformed<T, $target> {
            fn from(value: Transformed<T, $source>) -> Self {
                Self {
                    inner: value.inner,
                    transform: value.transform.into(),
                }
            }
        }
    };
}

impl_transformed_widening!(Translation => Rigid);
impl_transformed_widening!(Translation => Similarity);
impl_transformed_widening!(Translation => DAffine3);
impl_transformed_widening!(Rigid => Similarity);
impl_transformed_widening!(Rigid => DAffine3);
impl_transformed_widening!(Similarity => DAffine3);
impl_transformed_widening!(Diag => DAffine3);

/// Extension methods for attaching an exact transform representation to a value.
pub trait TransformedExt: Sized {
    /// Return `self` wrapped with `transform` stored exactly as `G`.
    fn transformed<G>(self, transform: G) -> Transformed<Self, G> {
        Transformed::new(self, transform)
    }
}

impl<T> TransformedExt for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        core_item::{mesh_item::MeshItem, vitem::VItem},
        traits::{ScaleTransform, ShiftTransform, UniformScaleTransform},
    };
    use glam::{DQuat, Vec3, Vec4};

    fn assert_affine_eq(actual: DAffine3, expected: DAffine3) {
        assert!(
            actual
                .transform_point3(dvec3(0.3, -0.7, 1.1))
                .abs_diff_eq(expected.transform_point3(dvec3(0.3, -0.7, 1.1)), 1e-9)
        );
    }

    #[test]
    fn extract_mesh_item_composes_transform() {
        let mesh = MeshItem {
            transform: glam::Mat4::from_translation(Vec3::X),
            ..Default::default()
        };
        let wrapped = mesh.transformed(Translation(dvec3(0.0, 2.0, 0.0)));
        let items = wrapped.extract();
        match &items[0] {
            CoreItem::MeshItem(mesh) => {
                assert_eq!(
                    mesh.transform,
                    glam::Mat4::from_translation(Vec3::new(1.0, 2.0, 0.0))
                );
                assert_eq!(mesh.points, vec![Vec3::ZERO; 3]);
            }
            _ => panic!("expected MeshItem"),
        }
    }

    #[test]
    fn extract_vitem_composes_transform_without_baking_points() {
        let vitem = VItem {
            points: vec![Vec4::new(1.0, 0.0, 0.0, 0.0)],
            normal: Some(Vec3::new(1.0, 1.0, 0.0).normalize()),
            ..Default::default()
        };
        let wrapped = vitem.transformed(DAffine3::from_scale_rotation_translation(
            dvec3(2.0, 1.0, 1.0),
            DQuat::IDENTITY,
            dvec3(0.0, 2.0, 0.0),
        ));
        let items = wrapped.extract();
        match &items[0] {
            CoreItem::VItem(vitem) => {
                // Points and the local plane normal stay untouched.
                assert_eq!(vitem.points[0], Vec4::new(1.0, 0.0, 0.0, 0.0));
                assert_eq!(vitem.normal, Some(Vec3::new(1.0, 1.0, 0.0).normalize()));
                assert_eq!(
                    vitem.transform,
                    glam::Mat4::from_scale_rotation_translation(
                        Vec3::new(2.0, 1.0, 1.0),
                        glam::Quat::IDENTITY,
                        Vec3::new(0.0, 2.0, 0.0),
                    )
                );
            }
            _ => panic!("expected VItem"),
        }
    }

    #[test]
    fn nested_transforms_compose_inside_out() {
        let inner = VItem {
            points: vec![Vec4::new(1.0, 0.0, 0.0, 0.0)],
            ..Default::default()
        }
        .transformed(Translation(DVec3::X));
        let outer = inner.transformed(Diag(DVec3::splat(2.0)));
        match &outer.extract()[0] {
            CoreItem::VItem(vitem) => {
                // Local data is preserved; the world placement moves to
                // scale(2) * translate(x).
                assert_eq!(vitem.points[0], Vec4::new(1.0, 0.0, 0.0, 0.0));
                assert_eq!(vitem.transform.w_axis.truncate(), Vec3::new(2.0, 0.0, 0.0));
                assert_eq!(vitem.transform.x_axis.truncate(), Vec3::new(2.0, 0.0, 0.0));
            }
            _ => panic!("expected VItem"),
        }
    }

    #[test]
    fn generic_wrapper_composes_outer_and_inner_in_order() {
        let mut wrapped = ().transformed(DAffine3::IDENTITY);
        wrapped.compose_outer(Translation(DVec3::X));
        wrapped.compose_inner(Diag(DVec3::splat(2.0)));
        assert_affine_eq(
            wrapped.transform,
            DAffine3::from_translation(DVec3::X) * DAffine3::from_scale(DVec3::splat(2.0)),
        );

        wrapped.apply(Translation(DVec3::Y));
        assert_affine_eq(
            wrapped.transform,
            DAffine3::from_translation(DVec3::Y)
                * DAffine3::from_translation(DVec3::X)
                * DAffine3::from_scale(DVec3::splat(2.0)),
        );
    }

    #[test]
    fn subgroup_operations_keep_the_same_storage_type() {
        fn require_similarity<T>(_: &Transformed<T, Similarity>) {}

        let mut wrapped = ().transformed(Similarity::IDENTITY);
        wrapped.shift(DVec3::X).scale_uniform(2.0);
        require_similarity(&wrapped);
        assert_eq!(wrapped.transform.scale, 2.0);
        assert_eq!(wrapped.transform.translation, dvec3(2.0, 0.0, 0.0));
    }

    #[test]
    fn explicit_widening_preserves_numeric_transform() {
        let translation = ().transformed(Translation(dvec3(1.0, 2.0, 3.0)));
        let rigid: Transformed<_, Rigid> = translation.clone().into();
        let similarity: Transformed<_, Similarity> = translation.clone().into();
        let affine: Transformed<_, DAffine3> = translation.into();
        assert_affine_eq(rigid.transform.into(), affine.transform);
        assert_affine_eq(similarity.transform.into(), affine.transform);

        let rigid = ().transformed(Rigid {
            rotation: DQuat::from_rotation_z(0.7),
            translation: DVec3::Y,
        });
        let similarity: Transformed<_, Similarity> = rigid.clone().into();
        let affine: Transformed<_, DAffine3> = rigid.into();
        assert_affine_eq(similarity.transform.into(), affine.transform);

        let similarity = ().transformed(Similarity {
            scale: 2.5,
            rotation: DQuat::from_rotation_x(0.4),
            translation: DVec3::Z,
        });
        let affine: Transformed<_, DAffine3> = similarity.into();
        assert_affine_eq(
            affine.transform,
            DAffine3::from_scale_rotation_translation(
                DVec3::splat(2.5),
                DQuat::from_rotation_x(0.4),
                DVec3::Z,
            ),
        );

        let diagonal = ().transformed(Diag(dvec3(2.0, 0.0, -3.0)));
        let affine_diagonal: Transformed<_, DAffine3> = diagonal.into();
        assert_eq!(affine_diagonal.transform.matrix3.y_axis, DVec3::ZERO);
        assert_eq!(affine_diagonal.transform.matrix3.z_axis.z, -3.0);
    }

    #[test]
    fn bake_uses_the_exact_storage_group() {
        let wrapped = dvec3(1.0, 0.0, 0.0).transformed(Translation(dvec3(0.0, 2.0, 0.0)));
        assert_eq!(wrapped.bake(), dvec3(1.0, 2.0, 0.0));
    }

    #[test]
    fn interpolation_lerps_inner_and_same_group_independently() {
        let a = 0.0f64.transformed(Similarity::IDENTITY);
        let b = 2.0f64.transformed(Similarity {
            scale: 3.0,
            rotation: DQuat::IDENTITY,
            translation: dvec3(2.0, 0.0, 0.0),
        });
        let mid = a.lerp(&b, 0.5);
        assert_eq!(mid.inner, 1.0);
        assert_eq!(mid.transform.scale, 2.0);
        assert_eq!(mid.transform.translation, DVec3::X);
    }

    #[test]
    fn aabb_converts_transform_at_geometry_boundary() {
        let wrapped = dvec3(1.0, 1.0, 1.0).transformed(Translation(dvec3(10.0, 0.0, 0.0)));
        assert_eq!(wrapped.aabb(), [dvec3(11.0, 1.0, 1.0); 2]);
    }

    #[test]
    fn centroid_locates_in_inner_space_then_applies_external_transform() {
        let wrapped = dvec3(1.0, 2.0, 3.0).transformed(Translation(dvec3(4.0, 5.0, 6.0)));
        assert_eq!(
            crate::anchor::Centroid.locate(&wrapped),
            dvec3(5.0, 7.0, 9.0)
        );
    }

    #[test]
    fn scale_operation_is_available_for_affine_storage() {
        let mut wrapped = ().transformed(DAffine3::IDENTITY);
        wrapped.scale(DVec3::splat(2.0));
        assert_eq!(wrapped.transform, DAffine3::from_scale(DVec3::splat(2.0)));
    }

    #[test]
    fn map_inner_maps_the_wrapped_item() {
        let wrapped = VItem::default()
            .transformed(Translation(DVec3::X))
            .map_inner(|item: VItem| item.points.len());
        assert_eq!(wrapped.inner, 3);
        assert_eq!(wrapped.transform, Translation(DVec3::X));
    }

    #[test]
    fn map_transform_converts_storage_with_function() {
        fn to_affine(translation: Translation) -> DAffine3 {
            translation.into()
        }

        let wrapped = 42u32
            .transformed(Translation(DVec3::X))
            .map_transform(to_affine);
        assert_eq!(wrapped.inner, 42);
        assert_affine_eq(wrapped.transform, DAffine3::from_translation(DVec3::X));
    }

    #[test]
    fn map_transform_converts_storage_with_closure() {
        let wrapped = 42u32
            .transformed(Translation(DVec3::X))
            .map_transform(Similarity::from);
        assert_eq!(wrapped.inner, 42);
        assert_eq!(wrapped.transform.translation, DVec3::X);
    }

    #[test]
    fn map_transform_result_can_be_extracted_as_affine() {
        let vitem = VItem {
            points: vec![Vec4::new(1.0, 0.0, 0.0, 0.0)],
            ..Default::default()
        };
        let wrapped = vitem
            .transformed(Translation(DVec3::X))
            .map_transform(DAffine3::from);

        match &wrapped.extract()[0] {
            CoreItem::VItem(vitem) => {
                assert_eq!(vitem.points[0], Vec4::new(1.0, 0.0, 0.0, 0.0));
                assert_eq!(vitem.transform.w_axis.truncate(), Vec3::X);
            }
            _ => panic!("expected VItem"),
        }
    }
}
