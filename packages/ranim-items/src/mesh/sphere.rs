//! Sphere — a sphere mesh item.

use std::f64::consts::{PI, TAU};

use ranim_core::{
    Extract,
    anchor::Aabb,
    color::{self, AlphaColor, Srgb},
    core_item::{CoreItem, mesh_item::MeshItem},
    glam::{DMat4, DVec3},
    traits::{FillColor, Interpolatable, Opacity, ShiftTransform},
};

use super::{Surface, generate_grid_indices};

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

    /// Generate the sphere mesh points for the current radius and resolution.
    fn generate_points(&self) -> Vec<DVec3> {
        let (nu, nv) = self.resolution;
        let r = self.radius;
        let mut points = Vec::with_capacity((nu * nv) as usize);
        for i in 0..nu {
            let u = TAU * (i as f64 / (nu - 1) as f64);
            for j in 0..nv {
                let v = PI * (j as f64 / (nv - 1) as f64);
                let x = r * u.cos() * v.sin();
                let y = r * u.sin() * v.sin();
                let z = r * (-v.cos());
                points.push(DVec3::new(x, y, z));
            }
        }
        points
    }

    /// Convert this sphere to a [`Surface`].
    ///
    /// Useful when you need point-level morph animations between a sphere
    /// and another surface.
    pub fn to_surface(&self) -> Surface {
        let points = self.generate_points();
        let triangle_indices = generate_grid_indices(self.resolution.0, self.resolution.1);
        Surface {
            resolution: self.resolution,
            vertex_colors: vec![self.fill_rgba; points.len()],
            transform: DMat4::from_translation(self.center),
            points,
            triangle_indices,
        }
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
        let points = self.generate_points();
        let triangle_indices = generate_grid_indices(self.resolution.0, self.resolution.1);
        buf.push(CoreItem::MeshItem(MeshItem {
            points: points.iter().map(|p| p.as_vec3()).collect(),
            triangle_indices,
            transform: DMat4::from_translation(self.center).as_mat4(),
            vertex_colors: vec![self.fill_rgba.into(); points.len()],
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ranim_core::glam::dvec3;

    #[test]
    fn test_sphere_points_on_sphere() {
        let sphere = Sphere::new(2.0).with_resolution((11, 6));
        let points = sphere.generate_points();

        // All points should be at distance ~radius from origin
        for p in &points {
            let dist = p.length();
            assert!(
                (dist - 2.0).abs() < 1e-10,
                "point {:?} has distance {} from origin, expected 2.0",
                p,
                dist
            );
        }
    }

    #[test]
    fn test_sphere_center_to_transform() {
        let sphere = Sphere::new(1.0).with_center(dvec3(1.0, 2.0, 3.0));
        let surface = sphere.to_surface();
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
        let surface = sphere.to_surface();
        assert_eq!(surface.points.len(), 25);
        assert_eq!(surface.resolution, (5, 5));
        assert_eq!(
            surface.transform,
            DMat4::from_translation(dvec3(1.0, 0.0, 0.0))
        );
    }
}
