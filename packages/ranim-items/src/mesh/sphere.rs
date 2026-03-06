//! Sphere — a sphere mesh item.

use std::f64::consts::{PI, TAU};

use ranim_core::{
    Extract,
    anchor::Aabb,
    color::{self, AlphaColor, Srgb},
    core_item::CoreItem,
    glam::{DMat4, DVec3},
    traits::{FillColor, Interpolatable, Opacity, ShiftTransform, With},
};

use crate::mesh::MeshItem;

use super::Surface;

/// A sphere defined by center, radius, and resolution.
///
/// The sphere is parameterized as:
/// - `u ∈ [0, TAU]`, `v ∈ [0, PI]`
/// - `x = r * cos(u) * sin(v)`
/// - `y = r * sin(u) * sin(v)`
/// - `z = r * (-cos(v))`
#[derive(Debug, Clone, PartialEq)]
pub struct Sphere {
    /// Center of the sphere.
    pub center: DVec3,
    /// Radius of the sphere.
    pub radius: f64,
    /// Grid resolution `(nu, nv)`.
    pub resolution: (u32, u32),
    /// Fill color (with alpha).
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Sphere {
    /// Create a new sphere with the given radius, centered at the origin.
    pub fn new(radius: f64) -> Self {
        Self {
            center: DVec3::ZERO,
            radius,
            resolution: (101, 51),
            fill_rgba: color::palette::css::BLUE.with_alpha(1.0),
        }
    }

    /// Create a unit sphere (radius = 1).
    pub fn unit() -> Self {
        Self::new(1.0)
    }

    /// Set the center. Returns `self` for chaining.
    pub fn with_center(mut self, center: DVec3) -> Self {
        self.center = center;
        self
    }

    /// Set the resolution. Returns `self` for chaining.
    pub fn with_resolution(mut self, resolution: (u32, u32)) -> Self {
        self.resolution = resolution;
        self
    }

    /// Set the fill color. Returns `self` for chaining.
    pub fn with_fill_color(mut self, color: AlphaColor<Srgb>) -> Self {
        self.fill_rgba = color;
        self
    }

    /// Generate a point on the sphere using UV coordinates.
    pub fn points_uv_func(u: f64, v: f64, r: f64) -> DVec3 {
        Self::normals_uv_func(u, v) * r
    }

    /// Generate a normal on the sphere using UV coordinates.
    pub fn normals_uv_func(u: f64, v: f64) -> DVec3 {
        let x = u.cos() * v.sin();
        let y = u.sin() * v.sin();
        let z = -v.cos();
        DVec3::new(x, y, z)
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
        .with_transform(DMat4::from_translation(value.center))
        .with(|x| {
            x.set_fill_color(value.fill_rgba);
        })
    }
}

impl Interpolatable for Sphere {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            center: Interpolatable::lerp(&self.center, &target.center, t),
            radius: Interpolatable::lerp(&self.radius, &target.radius, t),
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

impl ShiftTransform for Sphere {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.center += offset;
        self
    }
}

impl Aabb for Sphere {
    fn aabb(&self) -> [DVec3; 2] {
        let r = DVec3::splat(self.radius);
        [self.center - r, self.center + r]
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
    use ranim_core::glam::dvec3;

    #[test]
    fn test_sphere_center_to_transform() {
        let sphere = Sphere::new(1.0).with_center(dvec3(1.0, 2.0, 3.0));
        let surface = Surface::from(sphere);
        assert_eq!(
            surface.transform,
            DMat4::from_translation(dvec3(1.0, 2.0, 3.0))
        );
    }

    #[test]
    fn test_sphere_aabb() {
        let sphere = Sphere::new(1.0).with_center(dvec3(1.0, 2.0, 3.0));
        let [min, max] = sphere.aabb();
        assert_eq!(min, dvec3(0.0, 1.0, 2.0));
        assert_eq!(max, dvec3(2.0, 3.0, 4.0));
    }

    #[test]
    fn test_sphere_shift() {
        let mut sphere = Sphere::new(1.0);
        sphere.shift(dvec3(1.0, 0.0, 0.0));
        assert_eq!(sphere.center, dvec3(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_sphere_interpolation() {
        let a = Sphere::new(1.0).with_center(dvec3(0.0, 0.0, 0.0));
        let b = Sphere::new(3.0).with_center(dvec3(2.0, 0.0, 0.0));
        let mid = a.lerp(&b, 0.5);
        assert!((mid.radius - 2.0).abs() < 1e-10);
        assert!((mid.center.x - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_sphere_to_surface() {
        let sphere = Sphere::new(1.0)
            .with_center(dvec3(1.0, 0.0, 0.0))
            .with_resolution((5, 5));
        let surface = Surface::from(sphere);
        assert_eq!(surface.vertices.len(), 25);
        assert_eq!(surface.resolution, (5, 5));
        assert_eq!(
            surface.transform,
            DMat4::from_translation(dvec3(1.0, 0.0, 0.0))
        );
    }
}
