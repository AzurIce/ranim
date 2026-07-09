use std::cmp::Ordering;

use glam::{DVec3, dvec3};
use itertools::Itertools;

use crate::{
    anchor::{BoundsAnchor, DBounds3, SemanticBounds},
    traits::{ShiftTransform, StrokeWidth},
};

/// A hint for deriving a scale factor from bounds.
#[derive(Debug, Clone, Copy)]
pub enum ScaleHint {
    /// Scale the mobject's X axis.
    X(f64),
    /// Scale the mobject's Y axis.
    Y(f64),
    /// Scale the mobject's Z axis.
    Z(f64),
    /// Scale the mobject's X axis, while other axes are scaled accordingly.
    ProportionalX(f64),
    /// Scale the mobject's Y axis, while other axes are scaled accordingly.
    ProportionalY(f64),
    /// Scale the mobject's Z axis, while other axes are scaled accordingly.
    ProportionalZ(f64),
}

/// A semantic scale factor type that can be derived from [`ScaleHint`].
///
/// For [`DVec3`], axis hints only scale that axis while proportional hints
/// scale all axes uniformly. For [`f64`], every hint resolves to a uniform
/// factor because a scalar factor cannot encode per-axis scaling.
pub trait ScaleFactor: Copy {
    /// The identity scale factor.
    fn identity() -> Self;

    /// Calculates a scale factor from a bounds size and a hint.
    fn from_hint(bounds_size: DVec3, hint: ScaleHint) -> Self;

    /// Component-wise/scalar minimum, used by [`ScaleExt::scale_to_min`].
    fn min(self, other: Self) -> Self;

    /// Component-wise/scalar maximum, used by [`ScaleExt::scale_to_max`].
    fn max(self, other: Self) -> Self;
}

impl ScaleFactor for DVec3 {
    fn identity() -> Self {
        DVec3::ONE
    }

    fn from_hint(bounds_size: DVec3, hint: ScaleHint) -> Self {
        match hint {
            ScaleHint::X(v) => dvec3(v / bounds_size.x, 1.0, 1.0),
            ScaleHint::Y(v) => dvec3(1.0, v / bounds_size.y, 1.0),
            ScaleHint::Z(v) => dvec3(1.0, 1.0, v / bounds_size.z),
            ScaleHint::ProportionalX(v) => DVec3::splat(v / bounds_size.x),
            ScaleHint::ProportionalY(v) => DVec3::splat(v / bounds_size.y),
            ScaleHint::ProportionalZ(v) => DVec3::splat(v / bounds_size.z),
        }
    }

    fn min(self, other: Self) -> Self {
        self.min(other)
    }

    fn max(self, other: Self) -> Self {
        self.max(other)
    }
}

impl ScaleFactor for f64 {
    fn identity() -> Self {
        1.0
    }

    fn from_hint(bounds_size: DVec3, hint: ScaleHint) -> Self {
        match hint {
            ScaleHint::X(v) | ScaleHint::ProportionalX(v) => v / bounds_size.x,
            ScaleHint::Y(v) | ScaleHint::ProportionalY(v) => v / bounds_size.y,
            ScaleHint::Z(v) | ScaleHint::ProportionalZ(v) => v / bounds_size.z,
        }
    }

    fn min(self, other: Self) -> Self {
        self.min(other)
    }

    fn max(self, other: Self) -> Self {
        self.max(other)
    }
}

/// Semantic scaling operations.
///
/// `Scale<S>` mutates an item by the semantic scale factor `S`. It does not
/// require an implementation to directly multiply every vertex or render
/// primitive. A circle can update its radius, a text-like item can update its
/// layout frame, and a raw vector item can scale its points.
///
/// The factor type is part of the item's semantic capability. Items that
/// support independent scene-axis scaling usually implement `Scale<DVec3>`;
/// uniform-only items such as circles can implement `Scale<f64>`.
///
/// `scale` applies about the current coordinate-space origin. Use
/// [`ScaleExt::scale_about_bounds`] or [`ScaleExt::scale_about`] when scaling
/// around an anchor in an operation bounds.
///
/// This trait is automatically implemented for [`DVec3`] and `[T]` where
/// `T: Scale<S>`.
pub trait Scale<S = DVec3> {
    /// Applies a semantic scale about the current coordinate-space origin.
    fn scale(&mut self, scale: S) -> &mut Self;
}

impl Scale<DVec3> for DVec3 {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        *self *= scale;
        self
    }
}

impl<T, S> Scale<S> for [T]
where
    T: Scale<S>,
    S: Copy,
{
    fn scale(&mut self, scale: S) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.scale(scale);
        });
        self
    }
}

impl<T, S> Scale<S> for Vec<T>
where
    T: Scale<S>,
    S: Copy,
{
    fn scale(&mut self, scale: S) -> &mut Self {
        self.as_mut_slice().scale(scale);
        self
    }
}

/// Useful extensions for semantic scaling operations.
///
/// This trait is implemented automatically for types that implement
/// [`Scale<S>`], you should not implement it yourself.
pub trait ScaleExt<S = DVec3>: Scale<S>
where
    S: ScaleFactor,
{
    /// Calculates the scale factor for a given explicit operation bounds.
    ///
    /// See [`ScaleHint`] for more details.
    fn calc_scale_factor_in_bounds(&self, bounds: DBounds3, hint: ScaleHint) -> S {
        S::from_hint(bounds.size(), hint)
    }

    /// Calculates the semantic scale factor for a given hint.
    ///
    /// This uses [`SemanticBounds::semantic_bounds`] as the default operation
    /// bounds. Use [`ScaleExt::calc_scale_factor_in_bounds`] when the operation
    /// should be based on an explicitly supplied bounds.
    ///
    /// See [`ScaleHint`] for more details.
    fn calc_scale_factor(&self, hint: ScaleHint) -> S
    where
        Self: SemanticBounds,
    {
        self.calc_scale_factor_in_bounds(self.semantic_bounds(), hint)
    }

    /// Scales the item about an anchor in explicit operation bounds.
    ///
    /// The supplied `bounds` is only the reference frame for locating the anchor.
    /// It can be the item's semantic bounds, extracted primitive bounds, or any
    /// custom bounds chosen by the caller.
    fn scale_about_bounds(&mut self, bounds: DBounds3, anchor: BoundsAnchor, scale: S) -> &mut Self
    where
        Self: ShiftTransform,
    {
        let origin = anchor.locate_in(bounds);
        self.shift(-origin);
        self.scale(scale);
        self.shift(origin)
    }

    /// Scales the item about an anchor in its semantic bounds.
    fn scale_about(&mut self, anchor: BoundsAnchor, scale: S) -> &mut Self
    where
        Self: SemanticBounds + ShiftTransform,
    {
        self.scale_about_bounds(self.semantic_bounds(), anchor, scale)
    }

    /// Scales the item so the explicit operation bounds match a given hint.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to_bounds(&mut self, bounds: DBounds3, hint: ScaleHint) -> &mut Self {
        self.scale(self.calc_scale_factor_in_bounds(bounds, hint));
        self
    }

    /// Scales the item so its semantic bounds match a given hint.
    ///
    /// This uses [`SemanticBounds::semantic_bounds`] as the default operation
    /// bounds. Use [`ScaleExt::scale_to_bounds`] when the operation should be
    /// based on an explicitly supplied bounds.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to(&mut self, hint: ScaleHint) -> &mut Self
    where
        Self: SemanticBounds,
    {
        self.scale(self.calc_scale_factor(hint));
        self
    }

    /// Scales the item to the minimum scale factor from the hints and explicit
    /// operation bounds.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to_min_bounds(&mut self, bounds: DBounds3, hints: &[ScaleHint]) -> &mut Self {
        let scale = hints
            .iter()
            .map(|hint| self.calc_scale_factor_in_bounds(bounds, *hint))
            .reduce(|a, b| a.min(b))
            .unwrap_or_else(S::identity);
        self.scale(scale);
        self
    }

    /// Scales the item to the minimum semantic scale factor from the hints.
    ///
    /// This uses [`SemanticBounds::semantic_bounds`] as the default operation
    /// bounds. Use [`ScaleExt::scale_to_min_bounds`] when the operation should
    /// be based on an explicitly supplied bounds.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to_min(&mut self, hints: &[ScaleHint]) -> &mut Self
    where
        Self: SemanticBounds,
    {
        let scale = hints
            .iter()
            .map(|hint| self.calc_scale_factor(*hint))
            .reduce(|a, b| a.min(b))
            .unwrap_or_else(S::identity);
        self.scale(scale);
        self
    }

    /// Scales the item to the maximum scale factor from the hints and explicit
    /// operation bounds.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to_max_bounds(&mut self, bounds: DBounds3, hints: &[ScaleHint]) -> &mut Self {
        let scale = hints
            .iter()
            .map(|hint| self.calc_scale_factor_in_bounds(bounds, *hint))
            .reduce(|a, b| a.max(b))
            .unwrap_or_else(S::identity);
        self.scale(scale);
        self
    }

    /// Scales the item to the maximum semantic scale factor from the hints.
    ///
    /// This uses [`SemanticBounds::semantic_bounds`] as the default operation
    /// bounds. Use [`ScaleExt::scale_to_max_bounds`] when the operation should
    /// be based on an explicitly supplied bounds.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to_max(&mut self, hints: &[ScaleHint]) -> &mut Self
    where
        Self: SemanticBounds,
    {
        let scale = hints
            .iter()
            .map(|hint| self.calc_scale_factor(*hint))
            .reduce(|a, b| a.max(b))
            .unwrap_or_else(S::identity);
        self.scale(scale);
        self
    }
}

impl<T, S> ScaleExt<S> for T
where
    T: Scale<S> + ?Sized,
    S: ScaleFactor,
{
}

/// A trait for scaling operations with stroke width.
pub trait ScaleStrokeExt: Scale<DVec3> + StrokeWidth {
    /// Scale the item with stroke width (at origin).
    fn scale_with_stroke(&mut self, scale: DVec3) -> &mut Self {
        self.scale(scale);

        let scales = [scale.x, scale.y, scale.z];
        let idx = scales
            .iter()
            .map(|x: &f64| if *x > 1.0 { *x } else { 1.0 / *x })
            .position_max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap_or(0);
        let scale = scales[idx];
        self.apply_stroke_func(|widths| widths.iter_mut().for_each(|w| w.0 *= scale as f32));
        self
    }
    /// Scale the item to a given hint and explicit operation bounds with stroke
    /// width.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to_bounds_with_stroke(&mut self, bounds: DBounds3, hint: ScaleHint) -> &mut Self {
        let scale = self.calc_scale_factor_in_bounds(bounds, hint);
        self.scale_with_stroke(scale)
    }

    /// Scale the item to a given semantic-bounds hint with stroke width.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to_with_stroke(&mut self, hint: ScaleHint) -> &mut Self
    where
        Self: SemanticBounds,
    {
        let scale = self.calc_scale_factor(hint);
        self.scale_with_stroke(scale)
    }
}

impl<T: Scale<DVec3> + StrokeWidth + ?Sized> ScaleStrokeExt for T {}
