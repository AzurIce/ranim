//! Surface — a parametric surface mesh item.

use ranim_core::{
    Extract,
    color::{self, AlphaColor, Srgb},
    components::rgba::Rgba,
    core_item::CoreItem,
    glam::{DMat4, DVec3},
    traits::{FillColor, Interpolatable, Opacity},
};

use crate::mesh::MeshItem;

use super::{compute_smooth_normals, generate_grid_indices};

/// Linearly interpolate a color from a sorted colorscale based on a value.
fn colorscale_lookup(colorscale: &[(AlphaColor<Srgb>, f64)], value: f64) -> AlphaColor<Srgb> {
    if colorscale.is_empty() {
        return color::palette::css::WHITE.with_alpha(1.0);
    }
    if value <= colorscale[0].1 {
        return colorscale[0].0;
    }
    if value >= colorscale[colorscale.len() - 1].1 {
        return colorscale[colorscale.len() - 1].0;
    }
    for i in 0..colorscale.len() - 1 {
        let (c0, v0) = colorscale[i];
        let (c1, v1) = colorscale[i + 1];
        if value >= v0 && value <= v1 {
            let t = ((value - v0) / (v1 - v0)) as f32;
            let [r0, g0, b0, a0] = c0.components;
            let [r1, g1, b1, a1] = c1.components;
            return AlphaColor::new([
                r0 + (r1 - r0) * t,
                g0 + (g1 - g0) * t,
                b0 + (b1 - b0) * t,
                a0 + (a1 - a0) * t,
            ]);
        }
    }
    colorscale[colorscale.len() - 1].0
}

/// A parametric surface defined by pre-generated mesh data.
///
/// Vertices are stored in row-major order: `points[i * nv + j]` where
/// `i` is the u-index and `j` is the v-index.
///
/// By default, vertex normals are all-zero, which causes flat shading.
/// To enable smooth shading, call [`Self::with_smooth_normals`] or [`Self::update_smooth_normals`] to update the normals.
#[derive(Debug, Clone, PartialEq)]
pub struct Surface {
    /// Vertices — `nu * nv` points in row-major order.
    pub vertices: Vec<DVec3>,
    /// Per-vertex colors.
    pub vertex_colors: Vec<AlphaColor<Srgb>>,
    /// Per-vertex normals for smooth shading. All-zero → flat shading.
    pub vertex_normals: Vec<DVec3>,
    /// Triangle indices — `6 * (nu-1) * (nv-1)` entries.
    pub triangle_indices: Vec<u32>,
    /// Grid resolution `(nu, nv)`.
    pub resolution: (u32, u32),
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

        let vertex_colors = vec![color::palette::css::BLUE.with_alpha(1.0); points.len()];
        let vertex_normals = vec![DVec3::ZERO; points.len()];
        Self {
            vertices: points,
            triangle_indices,
            resolution,
            vertex_colors,
            vertex_normals,
            transform: DMat4::IDENTITY,
        }
    }

    /// Set per-vertex colors. Returns `self` for chaining.
    pub fn with_vertex_colors(mut self, colors: Vec<AlphaColor<Srgb>>) -> Self {
        self.vertex_colors = colors;
        self
    }

    /// Set per-vertex colors by mapping the Z coordinate of each vertex through a colorscale.
    ///
    /// `colorscale` is a list of `(color, z_value)` pairs sorted by ascending `z_value`.
    /// The vertex color is linearly interpolated between adjacent entries.
    pub fn with_fill_by_z(mut self, colorscale: &[(AlphaColor<Srgb>, f64)]) -> Self {
        let colors = self
            .vertices
            .iter()
            .map(|p| colorscale_lookup(colorscale, p.z))
            .collect();
        self.vertex_colors = colors;
        self
    }

    /// Set the transform matrix. Returns `self` for chaining.
    pub fn with_transform(mut self, transform: DMat4) -> Self {
        self.transform = transform;
        self
    }

    /// Update per-vertex normals to smooth shading. Returns `self` for chaining.
    pub fn with_smooth_normals(mut self) -> Self {
        self.update_smooth_normals();
        self
    }
    /// Update per-vertex normals to smooth shading.
    pub fn update_smooth_normals(&mut self) -> &mut Self {
        self.vertex_normals = compute_smooth_normals(&self.vertices, &self.triangle_indices);
        self
    }
}

impl Interpolatable for Surface {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            vertices: self.vertices.lerp(&target.vertices, t),
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
            vertex_colors: self.vertex_colors.lerp(&target.vertex_colors, t),
            vertex_normals: self.vertex_normals.lerp(&target.vertex_normals, t),
            transform: Interpolatable::lerp(&self.transform, &target.transform, t),
        }
    }
}

impl FillColor for Surface {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        // TODO: make it better
        let Rgba(rgba) = self
            .vertex_colors
            .first()
            .cloned()
            .map(Rgba::from)
            .unwrap_or_default();
        AlphaColor::new([rgba.x, rgba.y, rgba.z, rgba.w])
    }

    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.vertex_colors.fill(color);
        self
    }

    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.vertex_colors
            .iter_mut()
            .for_each(|x| *x = x.with_alpha(opacity));
        self
    }
}

impl Opacity for Surface {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.set_fill_opacity(opacity)
    }
}

impl From<Surface> for MeshItem {
    fn from(value: Surface) -> Self {
        MeshItem {
            points: value
                .vertices
                .iter()
                .map(|p| p.as_vec3())
                .collect::<Vec<_>>()
                .into(),
            triangle_indices: value.triangle_indices,
            transform: value.transform.as_mat4(),
            vertex_colors: value
                .vertex_colors
                .into_iter()
                .map(Rgba::from)
                .collect::<Vec<_>>()
                .into(),
            vertex_normals: value
                .vertex_normals
                .iter()
                .map(|n| n.as_vec3())
                .collect::<Vec<_>>()
                .into(),
        }
    }
}

impl Extract for Surface {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        MeshItem::from(self.clone()).extract_into(buf);
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
        assert_eq!(surface.vertices.len(), 9);
        assert_eq!(surface.triangle_indices.len(), 24);
        assert_eq!(surface.resolution, (3, 3));

        // Check corners
        assert_eq!(surface.vertices[0], dvec3(0.0, 0.0, 0.0));
        assert_eq!(surface.vertices[2], dvec3(0.0, 1.0, 0.0));
        assert_eq!(surface.vertices[6], dvec3(1.0, 0.0, 0.0));
        assert_eq!(surface.vertices[8], dvec3(1.0, 1.0, 0.0));
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
        for p in &mid.vertices {
            assert!((p.z - 0.5).abs() < 1e-10);
        }
    }
}
