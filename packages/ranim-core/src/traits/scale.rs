use std::cmp::Ordering;

use glam::{DVec3, dvec3};
use itertools::Itertools;

use crate::{
    anchor::{Aabb, AabbPoint, Locate},
    traits::StrokeWidth,
    utils::wrap_point_func_with_point,
};

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
/// This trait is automatically implemented for [`DVec3`] and `[T]` where `T: Scale`.
pub trait Scale {
    /// Scale at a given point.
    fn scale_at_point(&mut self, scale: DVec3, point: DVec3) -> &mut Self;
}

impl Scale for DVec3 {
    fn scale_at_point(&mut self, scale: DVec3, point: DVec3) -> &mut Self {
        wrap_point_func_with_point(|p| *p *= scale, point)(self);
        self
    }
}

impl<T: Scale> Scale for [T] {
    fn scale_at_point(&mut self, scale: DVec3, point: DVec3) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.scale_at_point(scale, point);
        });
        self
    }
}

/// Useful extensions for scaling operations.
pub trait ScaleExt: Scale {
    /// Scale the item by a given scale at an anchor.
    ///
    /// See [`Locate`]
    fn scale_at<T>(&mut self, scale: DVec3, anchor: T) -> &mut Self
    where
        T: Locate<Self>,
    {
        let point = anchor.locate(self);
        self.scale_at_point(scale, point)
    }
    /// Scale the item by a given scale at [`AabbPoint::CENTER`].
    ///
    /// This is equivalent to [`Scale::scale_at`] with anchor of [`AabbPoint::CENTER`].
    fn scale(&mut self, scale: DVec3) -> &mut Self
    where
        AabbPoint: Locate<Self>,
    {
        self.scale_at(scale, AabbPoint::CENTER)
    }
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
    /// Scale the item to a given hint.
    ///
    /// See [`ScaleHint`].
    fn scale_to_at<T>(&mut self, hint: ScaleHint, anchor: T) -> &mut Self
    where
        Self: Aabb,
        T: Locate<Self>,
    {
        self.scale_at(self.calc_scale_ratio(hint), anchor);
        self
    }
    /// Scale the item to a given hint.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to(&mut self, hint: ScaleHint) -> &mut Self
    where
        Self: Aabb,
        AabbPoint: Locate<Self>,
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
        AabbPoint: Locate<Self>,
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
        AabbPoint: Locate<Self>,
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

impl<T: Scale + ?Sized> ScaleExt for T {}

/// A trait for scaling operations with stroke width.
pub trait ScaleStrokeExt: ScaleExt + StrokeWidth {
    /// Scale the item by a given scale at anchor with stroke width.
    fn scale_with_stroke_by_anchor<A>(&mut self, scale: DVec3, anchor_point: A) -> &mut Self
    where
        A: Locate<Self>,
    {
        self.scale_at(scale, anchor_point);

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
    /// Scale the item by a given scale with stroke width.
    fn scale_with_stroke(&mut self, scale: DVec3) -> &mut Self
    where
        AabbPoint: Locate<Self>,
    {
        self.scale_with_stroke_by_anchor(scale, AabbPoint::CENTER)
    }
    /// Scale the item to a given hint.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to_with_stroke(&mut self, hint: ScaleHint) -> &mut Self
    where
        Self: Aabb,
        AabbPoint: Locate<Self>,
    {
        let scale = self.calc_scale_ratio(hint);
        self.scale_with_stroke(scale)
    }
}

impl<T: ScaleExt + StrokeWidth + ?Sized> ScaleStrokeExt for T {}
