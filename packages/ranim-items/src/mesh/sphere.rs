//! Sphere — a sphere mesh item.

use std::f64::consts::{PI, TAU};

use ranim_core::{
    Extract,
    anchor::Aabb,
    color::{self, AlphaColor, Srgb},
    core_item::CoreItem,
    glam::DVec3,
    traits::{FillColor, Interpolatable, Opacity, With},
};

use super::Surface;
use crate::mesh::MeshItem;

/// A sphere primitive centered at the origin.
#[derive(Debug, Clone, PartialEq)]
pub struct Sphere {
    /// Sphere radius.
    pub radius: f64,
    /// UV mesh resolution `(u, v)`.
    pub resolution: (u32, u32),
    /// Sphere fill color.
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Sphere {
    /// Creates a sphere centered at the origin.
    pub fn new(radius: f64) -> Self {
        Self {
            radius,
            resolution: (101, 51),
            fill_rgba: color::palette::css::BLUE.with_alpha(1.0),
        }
    }
    /// Creates a unit sphere.
    pub fn unit() -> Self {
        Self::new(1.0)
    }
    /// Sets the UV mesh resolution.
    pub fn with_resolution(mut self, resolution: (u32, u32)) -> Self {
        self.resolution = resolution;
        self
    }
    /// Sets the fill color.
    pub fn with_fill_color(mut self, color: AlphaColor<Srgb>) -> Self {
        self.fill_rgba = color;
        self
    }
    /// Returns a point at spherical UV coordinates and radius `r`.
    pub fn points_uv_func(u: f64, v: f64, r: f64) -> DVec3 {
        Self::normals_uv_func(u, v) * r
    }
    /// Returns the unit normal at spherical UV coordinates.
    pub fn normals_uv_func(u: f64, v: f64) -> DVec3 {
        DVec3::new(u.cos() * v.sin(), u.sin() * v.sin(), -v.cos())
    }
}
impl From<Sphere> for MeshItem {
    fn from(value: Sphere) -> Self {
        Surface::from(value).into()
    }
}
impl From<Sphere> for Surface {
    fn from(value: Sphere) -> Self {
        Surface::from_uv_func(
            |u, v| Sphere::points_uv_func(u, v, value.radius),
            (0.0, TAU),
            (0.0, PI),
            value.resolution,
        )
        .with(|x| {
            x.set_fill_color(value.fill_rgba);
        })
    }
}
impl Interpolatable for Sphere {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            radius: self.radius.lerp(&target.radius, t),
            resolution: if t < 0.5 {
                self.resolution
            } else {
                target.resolution
            },
            fill_rgba: Interpolatable::lerp(&self.fill_rgba, &target.fill_rgba, t),
        }
    }
}
impl FillColor for Sphere {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.fill_rgba
    }
    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.fill_rgba = color;
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self
    }
}
impl Opacity for Sphere {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self
    }
}
impl Aabb for Sphere {
    fn aabb(&self) -> [DVec3; 2] {
        let r = DVec3::splat(self.radius);
        [-r, r]
    }
}
impl Extract for Sphere {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        Surface::from(self.clone()).extract_into(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ranim_core::{glam::dvec3, prelude::TransformedExt, traits::Translation};
    #[test]
    fn sphere_surface_uses_canonical_local_vertices() {
        let surface = Surface::from(Sphere::new(1.0).with_resolution((5, 5)));
        assert_eq!(surface.vertices.len(), 25);
        assert_eq!(surface.resolution, (5, 5));
        assert!(surface.vertices[0].abs_diff_eq(Sphere::points_uv_func(0.0, 0.0, 1.0), 1e-10));
    }
    #[test]
    fn sphere_aabb_is_canonical_local_bounds() {
        let [min, max] = Sphere::new(1.0).aabb();
        assert_eq!(min, dvec3(-1.0, -1.0, -1.0));
        assert_eq!(max, dvec3(1.0, 1.0, 1.0));
    }
    #[test]
    fn transformed_sphere_owns_external_position() {
        let sphere = Sphere::new(1.0).transformed(Translation(dvec3(1.0, 2.0, 3.0)));
        let [min, max] = sphere.aabb();
        assert_eq!(min, dvec3(0.0, 1.0, 2.0));
        assert_eq!(max, dvec3(2.0, 3.0, 4.0));
    }
    #[test]
    fn test_sphere_interpolation() {
        let mid = Sphere::new(1.0).lerp(&Sphere::new(3.0), 0.5);
        assert!((mid.radius - 2.0).abs() < 1e-10);
    }
}
