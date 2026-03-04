//! Surface — a parametric surface mesh item.

use ranim_core::{
    Extract,
    color::{self, AlphaColor, Srgb},
    core_item::{CoreItem, mesh_item::MeshItem},
    glam::{DMat4, DVec3},
    traits::{FillColor, Interpolatable, Opacity},
};

use super::generate_grid_indices;

/// A parametric surface defined by pre-generated mesh data.
///
/// Vertices are stored in row-major order: `points[i * nv + j]` where
/// `i` is the u-index and `j` is the v-index.
#[derive(Debug, Clone, PartialEq)]
pub struct Surface {
    /// Vertices — `nu * nv` points in row-major order.
    pub points: Vec<DVec3>,
    /// Triangle indices — `6 * (nu-1) * (nv-1)` entries.
    pub triangle_indices: Vec<u32>,
    /// Grid resolution `(nu, nv)`.
    pub resolution: (u32, u32),
    /// Fill color (with alpha).
    pub fill_rgba: AlphaColor<Srgb>,
    /// Transform matrix applied when rendering.
    pub transform: DMat4,
}

impl Surface {
    /// Construct a surface by sampling `uv_func` over a uniform grid.
    ///
    /// `u_range` and `v_range` define the parameter domain.
    /// `resolution` `(nu, nv)` must each be >= 2.
    pub fn from_uv_func(
        uv_func: impl Fn(f64, f64) -> DVec3,
        u_range: (f64, f64),
        v_range: (f64, f64),
        resolution: (u32, u32),
    ) -> Self {
        let (nu, nv) = resolution;
        assert!(nu >= 2 && nv >= 2, "resolution must be >= (2, 2)");

        let mut points = Vec::with_capacity((nu * nv) as usize);
        for i in 0..nu {
            let u = u_range.0 + (u_range.1 - u_range.0) * (i as f64 / (nu - 1) as f64);
            for j in 0..nv {
                let v = v_range.0 + (v_range.1 - v_range.0) * (j as f64 / (nv - 1) as f64);
                points.push(uv_func(u, v));
            }
        }

        let triangle_indices = generate_grid_indices(nu, nv);

        Self {
            points,
            triangle_indices,
            resolution,
            fill_rgba: color::palette::css::BLUE.with_alpha(1.0),
            transform: DMat4::IDENTITY,
        }
    }

    /// Set the fill color. Returns `self` for chaining.
    pub fn with_fill_color(mut self, color: AlphaColor<Srgb>) -> Self {
        self.fill_rgba = color;
        self
    }

    /// Set the transform matrix. Returns `self` for chaining.
    pub fn with_transform(mut self, transform: DMat4) -> Self {
        self.transform = transform;
        self
    }
}

impl Interpolatable for Surface {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            points: self.points.lerp(&target.points, t),
            // TODO: better interpolation
            triangle_indices: if t < 0.5 {
                self.triangle_indices.clone()
            } else {
                target.triangle_indices.clone()
            },
            resolution: if t < 0.5 {
                self.resolution
            } else {
                target.resolution
            },
            fill_rgba: Interpolatable::lerp(&self.fill_rgba, &target.fill_rgba, t),
            transform: Interpolatable::lerp(&self.transform, &target.transform, t),
        }
    }
}

impl FillColor for Surface {
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

impl Opacity for Surface {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self
    }
}

impl Extract for Surface {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        buf.push(CoreItem::MeshItem(MeshItem {
            points: self.points.iter().map(|p| p.as_vec3()).collect(),
            triangle_indices: self.triangle_indices.clone(),
            transform: self.transform.as_mat4(),
            fill_rgba: self.fill_rgba.into(),
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ranim_core::glam::dvec3;

    #[test]
    fn test_flat_surface() {
        let surface =
            Surface::from_uv_func(|u, v| dvec3(u, v, 0.0), (0.0, 1.0), (0.0, 1.0), (3, 3));
        assert_eq!(surface.points.len(), 9);
        assert_eq!(surface.triangle_indices.len(), 24);
        assert_eq!(surface.resolution, (3, 3));

        // Check corners
        assert_eq!(surface.points[0], dvec3(0.0, 0.0, 0.0));
        assert_eq!(surface.points[2], dvec3(0.0, 1.0, 0.0));
        assert_eq!(surface.points[6], dvec3(1.0, 0.0, 0.0));
        assert_eq!(surface.points[8], dvec3(1.0, 1.0, 0.0));
    }

    #[test]
    fn test_surface_extract() {
        let surface =
            Surface::from_uv_func(|u, v| dvec3(u, v, 0.0), (0.0, 1.0), (0.0, 1.0), (2, 2));
        let items = surface.extract();
        assert_eq!(items.len(), 1);
        match &items[0] {
            CoreItem::MeshItem(mesh) => {
                assert_eq!(mesh.points.len(), 4);
                assert_eq!(mesh.triangle_indices.len(), 6);
            }
            _ => panic!("expected MeshItem"),
        }
    }

    #[test]
    fn test_surface_interpolation() {
        let a = Surface::from_uv_func(|u, v| dvec3(u, v, 0.0), (0.0, 1.0), (0.0, 1.0), (2, 2));
        let b = Surface::from_uv_func(|u, v| dvec3(u, v, 1.0), (0.0, 1.0), (0.0, 1.0), (2, 2));
        let mid = a.lerp(&b, 0.5);
        // z should be 0.5 for all points
        for p in &mid.points {
            assert!((p.z - 0.5).abs() < 1e-10);
        }
    }
}
