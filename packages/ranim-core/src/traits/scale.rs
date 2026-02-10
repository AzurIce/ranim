use std::cmp::Ordering;

use glam::{DVec3, dvec3};
use itertools::Itertools;

use crate::{
    anchor::{Aabb, AabbPoint, Locate},
    proj::ProjectionPlane,
    traits::{LocalCoordinate, Origin, StrokeWidth},
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

// TODO: Maybe add a derive macro for items with `Scale` or `ScaleByProj`?
/// Uniform scaling operations. (i.e. Scale ratio is the same for each axis)
pub trait ScaleUniform {
    /// Scale the item uniformly by a given scale at a given point.
    fn scale_uniform_at_point(&mut self, scale: f64, point: DVec3) -> &mut Self;
    /// Scale the item uniformly by a given scale at an anchor.
    ///
    /// See [`Locate`]
    fn scale_uniform_at<T>(&mut self, scale: f64, anchor: T) -> &mut Self
    where
        T: Locate<Self>,
    {
        let point = anchor.locate(self);
        self.scale_uniform_at_point(scale, point);
        self
    }

    /// Scale the item by a given scale at [`AabbPoint::CENTER`].
    fn scale_uniform_at_center(&mut self, scale: f64) -> &mut Self
    where
        AabbPoint: Locate<Self>,
    {
        self.scale_uniform_at(scale, AabbPoint::CENTER);
        self
    }
}

/// Uniform scaling operations relative to the item's origin.
pub trait ScaleUniformByOrigin: Origin {
    /// Scale the item uniformly by a given scale at the item's origin.
    fn scale_uniform(&mut self, scale: f64) -> &mut Self;
}

/// Uniform scaling in local coordinate system.
pub trait ScaleUniformLocal: LocalCoordinate + ScaleUniform {
    /// Scale the item by a given scale in the local coordinate system.
    fn scale_uniform_at_coord(&mut self, scale: f64, coord: DVec3) -> &mut Self {
        self.scale_uniform_at_point(scale, self.coord().c2p(coord))
    }
}

/// Scaling operations.
///
/// This trait is automatically implemented for [`DVec3`] and `[T]` where `T: Scale`.
pub trait Scale {
    /// Scale at a given point.
    fn scale_at_point(&mut self, scale: DVec3, point: DVec3) -> &mut Self;
}

/// Scaling operations relative to the item's origin.
pub trait ScaleByOrigin: Origin {
    /// Scale the item by a given scale at the item's origin.
    fn scale(&mut self, scale: DVec3) -> &mut Self;
}

/// Scaling operations with the definition of axes relative to a projection plane.
pub trait ScaleByProj {
    /// Scale at a given point with a projection plane.
    fn scale_by_proj_at_point(
        &mut self,
        scale: DVec3,
        point: DVec3,
        proj: ProjectionPlane,
    ) -> &mut Self;
}

/// Scaling operations in local coordinate system.
pub trait ScaleLocal: LocalCoordinate {
    /// Scale the item by a given scale in the local coordinate system.
    fn scale_local_at_coord(&mut self, scale: DVec3, coord: DVec3) -> &mut Self;
    /// Scale the item by a given scale in the local coordinate system at a given point.
    fn scale_local_at_point(&mut self, scale: DVec3, point: DVec3) -> &mut Self {
        self.scale_local_at_coord(scale, self.coord().c2p(point));
        self
    }
    /// Scale the item by a given scale in the local coordinate system at the item's origin.
    fn scale_local(&mut self, scale: DVec3) -> &mut Self {
        self.scale_local_at_coord(scale, DVec3::ZERO);
        self
    }
}

impl ScaleUniform for DVec3 {
    fn scale_uniform_at_point(&mut self, scale: f64, point: DVec3) -> &mut Self {
        wrap_point_func_with_point(|p| *p *= scale, point)(self);
        self
    }

    fn scale_uniform_at_center(&mut self, _scale: f64) -> &mut Self
    where
        AabbPoint: Locate<Self>,
    {
        self
    }
}

impl ScaleUniformByOrigin for DVec3 {
    fn scale_uniform(&mut self, _scale: f64) -> &mut Self {
        self
    }
}

impl Scale for DVec3 {
    fn scale_at_point(&mut self, scale: DVec3, point: DVec3) -> &mut Self {
        wrap_point_func_with_point(|p| *p *= scale, point)(self);
        self
    }
}

impl ScaleByProj for DVec3 {
    fn scale_by_proj_at_point(
        &mut self,
        scale: DVec3,
        point: DVec3,
        proj: ProjectionPlane,
    ) -> &mut Self {
        let (u, v) = proj.basis();
        let w = proj.normal();
        // let disp = self - point;
        wrap_point_func_with_point(
            |p| {
                let rel = dvec3(p.dot(u), p.dot(v), p.dot(w));
                let scaled = rel * scale;
                *p = u * scaled.x + v * scaled.y + w * scaled.z
            },
            point,
        )(self);
        self
    }
}

impl<T: ScaleUniform> ScaleUniform for [T] {
    fn scale_uniform_at_point(&mut self, scale: f64, point: DVec3) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.scale_uniform_at_point(scale, point);
        });
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

impl<T: ScaleByProj> ScaleByProj for [T] {
    fn scale_by_proj_at_point(
        &mut self,
        scale: DVec3,
        point: DVec3,
        proj: ProjectionPlane,
    ) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.scale_by_proj_at_point(scale, point, proj);
        });
        self
    }
}

/// Useful extensions for scaling operations.
///
/// This trait is implemented automatically for types that implement [`Scale`], you should not implement it yourself.
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
    /// This is equivalent to [`ScaleExt::scale_at`] with anchor of [`AabbPoint::CENTER`].
    fn scale_at_center(&mut self, scale: DVec3) -> &mut Self
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
        self.scale_at_center(self.calc_scale_ratio(hint));
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
        self.scale_at_center(scale);
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
        self.scale_at_center(scale);
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
