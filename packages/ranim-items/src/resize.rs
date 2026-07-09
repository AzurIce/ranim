//! Semantic resize implementations for built-in items.

use ranim_core::{
    anchor::{BoundsAnchor, DBounds3},
    glam::{DMat4, DVec2, DVec3, dvec2},
    traits::{
        MIN_RESIZE_EXTENT, Resize, clamped_resize_size, resize_by_bounds, resize_preserving_anchor,
        resize_scale, resize_xy_by_bounds,
    },
};

use crate::{
    mesh::{MeshItem, Sphere, Surface},
    vitem::{
        VItem,
        geometry::{
            Arc, ArcBetweenPoints, Circle, Ellipse, EllipticArc, Line, Parallelogram, Polygon,
            Rectangle, RegularPolygon, Square,
        },
    },
};

fn resize_point_in_bounds(point: DVec3, bounds: DBounds3, scale: DVec3) -> DVec3 {
    let local_point = bounds.coord_system().world_to_local_point(point);
    let local_min = bounds.local_min();
    let resized = local_min + (local_point - local_min) * scale;
    bounds.coord_system().local_to_world_point(resized)
}

macro_rules! impl_scalar_resize_from_dvec3 {
    ($ty:ty) => {
        impl Resize<f64> for $ty {
            fn resize_about_bounds(
                &mut self,
                bounds: DBounds3,
                anchor: BoundsAnchor,
                size: f64,
            ) -> &mut Self {
                Resize::<DVec3>::resize_about_bounds(self, bounds, anchor, DVec3::splat(size));
                self
            }
        }
    };
}

impl Resize<DVec3> for Rectangle {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: DVec3,
    ) -> &mut Self {
        let size = clamped_resize_size(size);
        resize_preserving_anchor(self, bounds, anchor, |rect| {
            rect.size = dvec2(size.x, size.y);
        });
        self
    }
}

impl_scalar_resize_from_dvec3!(Rectangle);

impl Resize<f64> for Circle {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: f64,
    ) -> &mut Self {
        let size = size.abs().max(MIN_RESIZE_EXTENT);
        resize_preserving_anchor(self, bounds, anchor, |circle| {
            circle.radius = size / 2.0;
        });
        self
    }
}

impl Resize<f64> for Square {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: f64,
    ) -> &mut Self {
        let size = size.abs().max(MIN_RESIZE_EXTENT);
        resize_preserving_anchor(self, bounds, anchor, |square| {
            square.size = size;
        });
        self
    }
}

impl Resize<DVec3> for Ellipse {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: DVec3,
    ) -> &mut Self {
        let size = clamped_resize_size(size);
        resize_preserving_anchor(self, bounds, anchor, |ellipse| {
            ellipse.radius = dvec2(size.x / 2.0, size.y / 2.0);
        });
        self
    }
}

impl_scalar_resize_from_dvec3!(Ellipse);

impl Resize<DVec3> for Line {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: DVec3,
    ) -> &mut Self {
        resize_by_bounds(self, bounds, anchor, size);
        self
    }
}

impl_scalar_resize_from_dvec3!(Line);

impl Resize<f64> for Arc {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: f64,
    ) -> &mut Self {
        let size = size.abs().max(MIN_RESIZE_EXTENT);
        resize_preserving_anchor(self, bounds, anchor, |arc| {
            arc.radius = size / 2.0;
        });
        self
    }
}

impl Resize<DVec3> for ArcBetweenPoints {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: DVec3,
    ) -> &mut Self {
        let size = clamped_resize_size(size);
        resize_preserving_anchor(self, bounds, anchor, |arc| {
            let scale = resize_scale(bounds.size(), size);
            arc.start = resize_point_in_bounds(arc.start, bounds, scale);
            arc.end = resize_point_in_bounds(arc.end, bounds, scale);
        });
        self
    }
}

impl_scalar_resize_from_dvec3!(ArcBetweenPoints);

impl Resize<DVec3> for EllipticArc {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: DVec3,
    ) -> &mut Self {
        let size = clamped_resize_size(size);
        resize_preserving_anchor(self, bounds, anchor, |arc| {
            arc.radius = dvec2(size.x / 2.0, size.y / 2.0);
        });
        self
    }
}

impl_scalar_resize_from_dvec3!(EllipticArc);

impl Resize<DVec3> for Parallelogram {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: DVec3,
    ) -> &mut Self {
        resize_by_bounds(self, bounds, anchor, size);
        self
    }
}

impl_scalar_resize_from_dvec3!(Parallelogram);

impl Resize<DVec3> for Polygon {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: DVec3,
    ) -> &mut Self {
        resize_by_bounds(self, bounds, anchor, size);
        self
    }
}

impl_scalar_resize_from_dvec3!(Polygon);

impl Resize<f64> for RegularPolygon {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: f64,
    ) -> &mut Self {
        let size = size.abs().max(MIN_RESIZE_EXTENT);
        resize_preserving_anchor(self, bounds, anchor, |polygon| {
            polygon.radius = size / 2.0;
        });
        self
    }
}

impl Resize<DVec3> for VItem {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: DVec3,
    ) -> &mut Self {
        resize_by_bounds(self, bounds, anchor, size);
        self
    }
}

impl_scalar_resize_from_dvec3!(VItem);

impl Resize<DVec3> for MeshItem {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: DVec3,
    ) -> &mut Self {
        resize_by_bounds(self, bounds, anchor, size);
        self
    }
}

impl_scalar_resize_from_dvec3!(MeshItem);

impl Resize<f64> for Sphere {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: f64,
    ) -> &mut Self {
        let size = size.abs().max(MIN_RESIZE_EXTENT);
        resize_preserving_anchor(self, bounds, anchor, |sphere| {
            sphere.radius = size / 2.0;
        });
        self
    }
}

impl Resize<DVec3> for Surface {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: DVec3,
    ) -> &mut Self {
        let current_size = clamped_resize_size(bounds.size());
        let scale = resize_scale(current_size, size);
        let origin = anchor.locate_in(bounds);
        let transform = DMat4::from_translation(origin)
            * DMat4::from_scale(scale)
            * DMat4::from_translation(-origin);
        self.transform = transform * self.transform;
        self
    }
}

impl_scalar_resize_from_dvec3!(Surface);

impl Resize<DVec2> for VItem {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: DVec2,
    ) -> &mut Self {
        resize_xy_by_bounds(self, bounds, anchor, size);
        self
    }
}

impl Resize<DVec2> for MeshItem {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: DVec2,
    ) -> &mut Self {
        resize_xy_by_bounds(self, bounds, anchor, size);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ranim_core::{
        glam::{DVec3, Vec3, dvec3},
        traits::{RotateTransform, ScaleExt, ScaleHint, SemanticBounds},
    };

    #[test]
    fn circle_static_resize_is_uniform() {
        let mut circle = Circle::new(1.0);
        circle.center = dvec3(2.0, 3.0, 0.5);
        circle.resize(4.0);

        assert_eq!(circle.radius, 2.0);
        assert_eq!(circle.center, dvec3(2.0, 3.0, 0.5));
    }

    #[test]
    fn rectangle_static_axes_resize_preserves_center_by_default() {
        let mut rect = Rectangle::new(1.0, 1.0);
        let center = rect.semantic_bounds_center();
        rect.resize(dvec3(3.0, 2.0, 1.0));

        assert_eq!(rect.size, dvec2(3.0, 2.0));
        assert_eq!(rect.semantic_bounds_center(), center);
    }

    #[test]
    fn resize_about_bounds_uses_explicit_anchor_reference() {
        let mut circle = Circle::new(1.0);
        let bounds = DBounds3::new(dvec3(10.0, 20.0, 0.0), dvec3(12.0, 22.0, 0.0));

        Resize::<f64>::resize_about_bounds(&mut circle, bounds, BoundsAnchor::MIN, 4.0);

        assert_eq!(circle.radius, 2.0);
        assert_eq!(circle.semantic_bounds().world_min(), bounds.world_min());
    }

    #[test]
    fn scalar_scale_to_works_for_uniform_items() {
        let mut circle = Circle::new(1.0);
        circle.center = dvec3(2.0, 0.0, 0.0);

        circle.scale_to(ScaleHint::Y(6.0));

        assert_eq!(circle.radius, 3.0);
        assert_eq!(circle.center, dvec3(6.0, 0.0, 0.0));
        assert_eq!(circle.semantic_bounds_size().x, 6.0);
        assert_eq!(circle.semantic_bounds_size().y, 6.0);
    }

    #[test]
    fn scalar_proportional_scale_to_uses_selected_axis() {
        let mut square = Square::new(2.0);
        square.center = dvec3(1.0, 0.0, 0.0);

        square.scale_to(ScaleHint::ProportionalX(8.0));

        assert_eq!(square.size, 8.0);
        assert_eq!(square.center, dvec3(4.0, 0.0, 0.0));
    }

    #[test]
    fn raw_vitem_min_anchor_resize_preserves_min_corner() {
        let mut item = VItem::from_vpoints(vec![
            dvec3(1.0, 2.0, 0.0),
            dvec3(1.5, 2.0, 0.0),
            dvec3(2.0, 3.0, 0.0),
        ]);
        let old_min = item.semantic_bounds().world_min();
        item.resize_about(BoundsAnchor(DVec3::NEG_ONE), dvec3(4.0, 6.0, 1.0));

        let bounds = item.semantic_bounds();
        let min = bounds.world_min();
        let max = bounds.world_max();
        assert!((min - old_min).length() < 1.0e-9);
        assert!(
            (dvec2(max.x, max.y) - (dvec2(old_min.x, old_min.y) + dvec2(4.0, 6.0))).length()
                < 1.0e-9
        );
    }

    #[test]
    fn mesh_min_anchor_resize_maps_current_semantic_bounds_to_size() {
        let mut item =
            MeshItem::from_vertices(vec![Vec3::new(1.0, 2.0, 1.0), Vec3::new(2.0, 4.0, 3.0)]);
        let old_min = item.semantic_bounds().world_min();
        item.resize_about(BoundsAnchor(DVec3::NEG_ONE), dvec3(10.0, 4.0, 8.0));

        let bounds = item.semantic_bounds();
        let min = bounds.world_min();
        let max = bounds.world_max();
        assert!((min - old_min).length() < 1.0e-9);
        assert!((max - (old_min + dvec3(10.0, 4.0, 8.0))).length() < 1.0e-6);
    }

    #[test]
    fn rotated_rectangle_bounds_cover_all_vertices() {
        let mut rect = Rectangle::new(4.0, 2.0);
        rect.rotate_on_z(std::f64::consts::FRAC_PI_4);

        let bounds = rect.semantic_bounds();
        assert!(bounds.size().x > 4.2);
        assert!(bounds.size().y > 4.2);
    }

    #[test]
    fn line_resize_preserves_direction_sign() {
        let mut line = Line::new(dvec3(2.0, 1.0, 0.0), dvec3(-1.0, -1.0, 0.0));
        line.resize_about(BoundsAnchor::MIN, dvec3(6.0, 4.0, 1.0));

        assert!(line.points[0].x > line.points[1].x);
        assert!(line.points[0].y > line.points[1].y);
        assert!((line.semantic_bounds().size().truncate() - dvec2(6.0, 4.0)).length() < 1.0e-9);
    }

    #[test]
    fn parallelogram_resize_preserves_skew_direction() {
        let mut parallelogram = Parallelogram::new(
            dvec3(-1.0, -1.0, 0.0),
            (dvec3(3.0, 0.0, 0.0), dvec3(1.0, 2.0, 0.0)),
        );
        parallelogram.resize(dvec3(6.0, 4.0, 1.0));

        assert!(parallelogram.axes.1.x > 0.0);
        assert!(
            (parallelogram.semantic_bounds().size().truncate() - dvec2(6.0, 4.0)).length() < 1.0e-9
        );
    }
}
