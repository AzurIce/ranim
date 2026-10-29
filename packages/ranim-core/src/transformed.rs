//! A wrapper that attaches a typed external transform to an item.

use std::ops::Range;

use glam::{DAffine3, DVec3, dvec3};

use crate::{
    anchor::{Aabb, Locate},
    color::{AlphaColor, Srgb},
    components::width::Width,
    traits::{
        Alignable, ApplyTransform, Diag, Empty, FillColor, Interpolatable, Opacity, Partial, Rigid,
        Similarity, StrokeColor, StrokeWidth, TransformGroup, Translation,
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
/// Evaluation preserves the typed wrapper. [`Aabb`] calculation converts `G`
/// to [`DAffine3`] at the geometry boundary. Projective transforms remain
/// outside the model-transform system.
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

impl<T: StrokeWidth, G> StrokeWidth for Transformed<T, G> {
    fn stroke_width(&self) -> f32 {
        self.inner.stroke_width()
    }

    fn apply_stroke_func(&mut self, f: impl for<'a> Fn(&'a mut [Width])) -> &mut Self {
        self.inner.apply_stroke_func(f);
        self
    }
}

impl<T: Partial, G: Clone> Partial for Transformed<T, G> {
    /// Take a partial slice of the wrapped item, keeping the pose.
    ///
    /// The range is forwarded to `inner`, so the partial geometry is taken in
    /// the item's own coordinates, while the stored transform `G` is cloned
    /// unchanged. The wrapper therefore keeps its placement while the visible
    /// geometry grows from the slice, which is what create-style animations
    /// expect.
    fn get_partial(&self, range: Range<f64>) -> Self {
        Transformed::new(self.inner.get_partial(range), self.transform.clone())
    }

    /// Take a closed partial slice of the wrapped item, keeping the pose.
    ///
    /// Like [`get_partial`](Self::get_partial), but the inner slice is closed
    /// so partially shown curves have caps at both ends; the wrapper's
    /// transform is preserved.
    fn get_partial_closed(&self, range: Range<f64>) -> Self {
        Transformed::new(self.inner.get_partial_closed(range), self.transform.clone())
    }
}

impl<T: Empty, G: TransformGroup> Empty for Transformed<T, G> {
    /// Create an empty placeholder of a wrapped item.
    ///
    /// The inner item contributes `T::empty()` while the pose is `G::identity()`:
    /// an empty placeholder carries no geometry, so there is no meaningful
    /// placement to preserve — identity keeps it neutral for composition until
    /// real content is placed on it.
    fn empty() -> Self {
        Transformed::new(T::empty(), G::identity())
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
        RanimScene,
        animation::{build::PlaybackExt, eval::Static},
        frame::EvaluatedFrame,
        traits::{ShiftTransform, UniformScaleTransform},
    };
    use glam::DQuat;

    #[derive(Debug, Clone, PartialEq)]
    struct LocalGeometry {
        points: Vec<DVec3>,
        transform: DAffine3,
    }

    fn assert_affine_eq(actual: DAffine3, expected: DAffine3) {
        assert!(actual.matrix3.abs_diff_eq(expected.matrix3, 1e-9));
        assert!(actual.translation.abs_diff_eq(expected.translation, 1e-9));
    }

    #[test]
    fn evaluation_preserves_the_typed_wrapper_and_local_geometry() {
        let geometry = LocalGeometry {
            points: vec![DVec3::ZERO; 3],
            transform: DAffine3::from_translation(DVec3::X),
        };
        let wrapped = geometry.transformed(Translation(dvec3(0.0, 2.0, 0.0)));
        let mut scene = RanimScene::new();
        scene.play(Static(wrapped.clone()).with_duration(1.0));
        let mut frame = EvaluatedFrame::new();
        scene.seal().eval_at_sec(0.5, &mut frame);

        let items = frame.items::<Transformed<LocalGeometry, Translation>>();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].value, wrapped);
        assert_eq!(
            items[0].value.inner.transform,
            DAffine3::from_translation(DVec3::X)
        );
        assert_eq!(items[0].value.inner.points, vec![DVec3::ZERO; 3]);
        assert!(frame.items::<LocalGeometry>().is_empty());
    }

    #[test]
    fn transforms_compose_inside_out() {
        let inner = vec![DVec3::X].transformed(Translation(DVec3::X));
        let outer = inner.transformed(Diag(DVec3::splat(2.0)));
        assert_eq!(outer.inner.inner, vec![DVec3::X]);
        assert_affine_eq(
            DAffine3::from(outer.transform) * DAffine3::from(outer.inner.transform),
            DAffine3::from_scale(DVec3::splat(2.0)) * DAffine3::from_translation(DVec3::X),
        );

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
    fn interpolation_lerps_inner_and_pose_by_storage_group() {
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

        let points = vec![DVec3::X];
        let a = points.clone().transformed(Translation(DVec3::X));
        let b = points.transformed(Translation(dvec3(-1.0, 2.0, 0.0)));
        let mid = a.lerp(&b, 0.5);
        // Local geometry never moves; only the pose interpolates.
        assert_eq!(mid.inner, vec![DVec3::X]);
        assert_affine_eq(
            DAffine3::from(mid.transform),
            DAffine3::from_translation(dvec3(0.0, 1.0, 0.0)),
        );

        let a = 42.0f64.transformed(Rigid::IDENTITY);
        let b = 42.0f64.transformed(Rigid::from_axis_angle(
            DVec3::Z,
            std::f64::consts::FRAC_PI_2,
        ));
        let mid = a.lerp(&b, 0.5);
        let rotated = mid.transform.rotation * DVec3::X;
        let expected = DQuat::from_axis_angle(DVec3::Z, std::f64::consts::FRAC_PI_4) * DVec3::X;
        assert!(rotated.abs_diff_eq(expected, 1e-9));
    }

    #[test]
    fn anchor_queries_apply_the_outer_pose() {
        let wrapped = dvec3(1.0, 1.0, 1.0).transformed(Translation(dvec3(10.0, 0.0, 0.0)));
        assert_eq!(wrapped.aabb(), [dvec3(11.0, 1.0, 1.0); 2]);

        let wrapped = dvec3(1.0, 2.0, 3.0).transformed(Translation(dvec3(4.0, 5.0, 6.0)));
        assert_eq!(
            crate::anchor::Centroid.locate(&wrapped),
            dvec3(5.0, 7.0, 9.0)
        );
    }

    #[test]
    fn map_operations_preserve_pose_and_support_storage_conversion() {
        let mapped = vec![DVec3::ZERO; 3]
            .transformed(Translation(DVec3::X))
            .map_inner(|points| points.len())
            .map_transform(DAffine3::from);
        assert_eq!(mapped.inner, 3);
        assert_eq!(mapped.transform, DAffine3::from_translation(DVec3::X));
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

    /// A minimal stand-in for the geometry traits: `ranim-core` itself has no
    /// concrete `Partial` type, so a partial slice is modelled by recording
    /// the requested range and whether it was closed.
    #[derive(Clone, Debug, PartialEq)]
    struct StubShape {
        /// The range this shape was last materialized from.
        range: Range<f64>,
        closed: bool,
    }

    impl Partial for StubShape {
        fn get_partial(&self, range: Range<f64>) -> Self {
            Self {
                range,
                closed: false,
            }
        }

        fn get_partial_closed(&self, range: Range<f64>) -> Self {
            Self {
                range,
                closed: true,
            }
        }
    }

    impl Empty for StubShape {
        fn empty() -> Self {
            Self {
                range: 0.0..0.0,
                closed: false,
            }
        }
    }

    #[test]
    fn partial_forwards_the_range_and_keeps_the_pose() {
        let wrapped = StubShape {
            range: 0.0..1.0,
            closed: false,
        }
        .transformed(Translation(DVec3::X));

        let partial = wrapped.get_partial(0.2..0.8);
        // The very same range reaches the inner item.
        assert_eq!(partial.inner.range, 0.2..0.8);
        assert!(!partial.inner.closed);
        // The stored pose is cloned untouched.
        assert_eq!(partial.transform, wrapped.transform);

        let closed = wrapped.get_partial_closed(0.3..0.6);
        assert_eq!(closed.inner.range, 0.3..0.6);
        assert!(closed.inner.closed);
        assert_eq!(closed.transform, wrapped.transform);
    }

    #[test]
    fn empty_wrapper_carries_an_identity_pose() {
        let wrapped: Transformed<StubShape, Translation> = Empty::empty();
        assert_eq!(wrapped.inner, StubShape::empty());
        assert_eq!(wrapped.transform, Translation(DVec3::ZERO));
    }
}
