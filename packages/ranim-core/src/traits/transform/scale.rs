use std::cmp::Ordering;

use glam::{dvec3, DVec3};
use itertools::Itertools;

use crate::{anchor::Aabb, traits::StrokeWidth};

/// A hint for scaling the mobject.
#[derive(Debug, Clone, Copy)]
pub enum ScaleHint {
    /// Scale the mobject's X axe
    X(f64),
    /// Scale the mobject's Y axe
    Y(f64),
    /// Scale the mobject's Z axe
    Z(f64),
    /// Scale the mobject's X axe, while other axes are scaled accordingly.
    PorportionalX(f64),
    /// Scale the mobject's Y axe, while other axes are scaled accordingly.
    PorportionalY(f64),
    /// Scale the mobject's Z axe, while other axes are scaled accordingly.
    PorportionalZ(f64),
}

/// Scaling operations.
///
/// This trait is automatically implemented for [`DVec3`] and `[T]` where `T: ScaleTransform`.
pub trait ScaleTransform {
    /// Scale at the origin.
    fn scale(&mut self, scale: DVec3) -> &mut Self;
}

impl ScaleTransform for DVec3 {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        *self *= scale;
        self
    }
}

impl<T: ScaleTransform> ScaleTransform for [T] {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.scale(scale);
        });
        self
    }
}

impl<T: ScaleTransform> ScaleTransform for Vec<T> {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.as_mut_slice().scale(scale);
        self
    }
}

/// Useful extensions for scaling operations.
///
/// This trait is implemented automatically for types that implement [`ScaleTransform`], you should not implement it yourself.
pub trait ScaleTransformExt: ScaleTransform {
    /// Calculate the scale ratio for a given hint.
    ///
    /// See [`ScaleHint`] for more details.
    fn calc_scale_ratio(&self, hint: ScaleHint) -> DVec3
    where
        Self: Aabb,
    {
        let aabb_size = self.aabb_size();
        match hint {
            ScaleHint::X(v) => dvec3(v / (aabb_size.x), 1.0, 1.0),
            ScaleHint::Y(v) => dvec3(1.0, v / (aabb_size.y), 1.0),
            ScaleHint::Z(v) => dvec3(1.0, 1.0, v / aabb_size.z),
            ScaleHint::PorportionalX(v) => DVec3::splat(v / aabb_size.x),
            ScaleHint::PorportionalY(v) => DVec3::splat(v / aabb_size.y),
            ScaleHint::PorportionalZ(v) => DVec3::splat(v / aabb_size.z),
        }
    }
    /// Scale the item to a given hint (at origin).
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to(&mut self, hint: ScaleHint) -> &mut Self
    where
        Self: Aabb,
    {
        self.scale(self.calc_scale_ratio(hint));
        self
    }
    /// Scale the item to the minimum scale ratio of each axis from the given hints.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to_min(&mut self, hints: &[ScaleHint]) -> &mut Self
    where
        Self: Aabb,
    {
        let scale = hints
            .iter()
            .map(|hint| self.calc_scale_ratio(*hint))
            .reduce(|a, b| a.min(b))
            .unwrap_or(DVec3::ONE);
        self.scale(scale);
        self
    }
    /// Scale the item to the maximum scale ratio of each axis from the given hints.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to_max(&mut self, hints: &[ScaleHint]) -> &mut Self
    where
        Self: Aabb,
    {
        let scale = hints
            .iter()
            .map(|hint| self.calc_scale_ratio(*hint))
            .reduce(|a, b| a.max(b))
            .unwrap_or(DVec3::ONE);
        self.scale(scale);
        self
    }
}

impl<T: ScaleTransform + ?Sized> ScaleTransformExt for T {}

/// A trait for scaling operations with stroke width.
pub trait ScaleStrokeTransformExt: ScaleTransform + StrokeWidth {
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
    /// Scale the item to a given hint with stroke width.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to_with_stroke(&mut self, hint: ScaleHint) -> &mut Self
    where
        Self: Aabb,
    {
        let scale = self.calc_scale_ratio(hint);
        self.scale_with_stroke(scale)
    }
}

impl<T: ScaleTransform + StrokeWidth + ?Sized> ScaleStrokeTransformExt for T {}
