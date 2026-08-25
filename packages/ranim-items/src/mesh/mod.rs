//! Mesh-based items (Surface, Sphere, etc.)

use ranim_core::{
    Extract,
    anchor::Aabb,
    color::{AlphaColor, Srgb},
    components::{PointVec, rgba::Rgba},
    core_item::CoreItem,
    glam::{DVec3, Mat4},
    traits::{Alignable, ApplyTransform, Empty, FillColor, Interpolatable, Opacity},
};

mod sphere;
mod surface;

pub use sphere::*;
pub use surface::*;

/// A high-level mesh item with per-vertex data wrapped in PointVec for animation support.
///
/// This struct uses [`PointVec`] to wrap vertex data, enabling proper alignment
/// and interpolation for animations. When extracted, it converts to the low-level
/// [`ranim_core::core_item::mesh_item::MeshItem`] for rendering.
///
/// The vertices are expressed in the mesh's local space. To place or animate the
/// mesh with a local-to-world transform, wrap it in
/// [`ranim_core::core_item::transformed::Transformed`].
#[derive(Debug, Clone, PartialEq)]
pub struct MeshItem {
    /// The vertices of the mesh (local space)
    pub points: PointVec<DVec3>,
    /// The triangle indices
    pub triangle_indices: Vec<u32>,
    /// Per-vertex colors
    pub vertex_colors: PointVec<Rgba>,
    /// Per-vertex normals for smooth shading.
    /// All-zero (or empty) → shader falls back to flat shading via `dpdx`/`dpdy`.
    pub vertex_normals: PointVec<DVec3>,
}

impl MeshItem {
    /// Create a MeshItem from vertices only (no indices, suitable for point clouds).
    pub fn from_vertices(points: Vec<DVec3>) -> Self {
        let len = points.len();
        Self {
            points: points.into(),
            triangle_indices: Vec::new(),
            vertex_colors: vec![Rgba::default(); len].into(),
            vertex_normals: vec![DVec3::ZERO; len].into(),
        }
    }

    /// Create a MeshItem from vertices and triangle indices.
    pub fn from_indexed_vertices(points: Vec<DVec3>, triangle_indices: Vec<u32>) -> Self {
        let len = points.len();
        Self {
            points: points.into(),
            triangle_indices,
            vertex_colors: vec![Rgba::default(); len].into(),
            vertex_normals: vec![DVec3::ZERO; len].into(),
        }
    }

    /// Set all vertex colors to the same value.
    pub fn with_color(mut self, color: AlphaColor<Srgb>) -> Self {
        let rgba: Rgba = color.into();
        self.vertex_colors = vec![rgba; self.points.len()].into();
        self
    }
}

impl From<MeshItem> for ranim_core::core_item::mesh_item::MeshItem {
    fn from(value: MeshItem) -> Self {
        Self {
            points: value.points.iter().map(|p| p.as_vec3()).collect(),
            triangle_indices: value.triangle_indices,
            transform: Mat4::IDENTITY,
            vertex_colors: value.vertex_colors.iter().copied().collect(),
            vertex_normals: value.vertex_normals.iter().map(|n| n.as_vec3()).collect(),
        }
    }
}

impl Extract for MeshItem {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        buf.push(CoreItem::MeshItem(self.clone().into()));
    }
}

impl<G: Into<ranim_core::glam::DAffine3>> ApplyTransform<G> for MeshItem {
    fn apply(&mut self, transform: G) -> &mut Self {
        let transform = transform.into();
        self.points.apply(transform);
        if transform.matrix3.determinant().abs() > 1e-12 {
            let normal_matrix = transform.matrix3.inverse().transpose();
            self.vertex_normals.iter_mut().for_each(|normal| {
                if let Some(unit) = (normal_matrix * *normal).try_normalize() {
                    *normal = unit;
                }
            });
        }
        self
    }
}

impl Alignable for MeshItem {
    fn is_aligned(&self, other: &Self) -> bool {
        self.points.is_aligned(&other.points)
            && self.vertex_colors.is_aligned(&other.vertex_colors)
            && self.vertex_normals.is_aligned(&other.vertex_normals)
    }

    fn align_with(&mut self, other: &mut Self) {
        self.points.align_with(&mut other.points);
        self.vertex_colors.align_with(&mut other.vertex_colors);
        self.vertex_normals.align_with(&mut other.vertex_normals);
    }
}

impl Interpolatable for MeshItem {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            points: self.points.lerp(&target.points, t),
            triangle_indices: if t < 0.5 {
                self.triangle_indices.clone()
            } else {
                target.triangle_indices.clone()
            },
            vertex_colors: self.vertex_colors.lerp(&target.vertex_colors, t),
            vertex_normals: self.vertex_normals.lerp(&target.vertex_normals, t),
        }
    }
}

impl FillColor for MeshItem {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        let Rgba(rgba) = self.vertex_colors.first().cloned().unwrap_or_default();
        AlphaColor::new([rgba.x, rgba.y, rgba.z, rgba.w])
    }

    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        let rgba: Rgba = color.into();
        self.vertex_colors.iter_mut().for_each(|c| *c = rgba);
        self
    }

    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.vertex_colors.set_opacity(opacity);
        self
    }
}

impl Opacity for MeshItem {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.vertex_colors.set_opacity(opacity);
        self
    }
}

impl Aabb for MeshItem {
    fn aabb(&self) -> [DVec3; 2] {
        if self.points.is_empty() {
            return [DVec3::ZERO, DVec3::ZERO];
        }

        // TODO: do some optimize and caching
        let mut min = self.points[0];
        let mut max = self.points[0];

        for &p in &self.points[1..] {
            min = min.min(p);
            max = max.max(p);
        }

        [min, max]
    }
}

impl Empty for MeshItem {
    fn empty() -> Self {
        Self {
            points: Vec::new().into(),
            triangle_indices: Vec::new(),
            vertex_colors: Vec::new().into(),
            vertex_normals: Vec::new().into(),
        }
    }
}

/// Compute smooth vertex normals from a triangle mesh.
///
/// Each face normal is weighted by the angle at the vertex before accumulation.
/// The result is normalized per vertex. Degenerate triangles are skipped.
pub fn compute_smooth_normals(points: &[DVec3], triangle_indices: &[u32]) -> Vec<DVec3> {
    let mut normals = vec![DVec3::ZERO; points.len()];

    for tri in triangle_indices.as_chunks::<3>().0 {
        let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        let (p0, p1, p2) = (points[i0], points[i1], points[i2]);

        let e01 = p1 - p0;
        let e02 = p2 - p0;
        let face_normal = e01.cross(e02);

        // Skip degenerate triangles
        if face_normal.length_squared() < 1e-20 {
            continue;
        }

        // Weight by angle at each vertex
        let e10 = p0 - p1;
        let e12 = p2 - p1;
        let e20 = p0 - p2;
        let e21 = p1 - p2;

        let angle0 = angle_between(e01, e02);
        let angle1 = angle_between(e10, e12);
        let angle2 = angle_between(e20, e21);

        normals[i0] += face_normal * angle0;
        normals[i1] += face_normal * angle1;
        normals[i2] += face_normal * angle2;
    }

    for n in &mut normals {
        let len = n.length();
        if len > 1e-10 {
            *n /= len;
        }
    }

    normals
}

/// Angle (in radians) between two vectors.
fn angle_between(a: DVec3, b: DVec3) -> f64 {
    let denom = a.length() * b.length();
    if denom < 1e-20 {
        return 0.0;
    }
    (a.dot(b) / denom).clamp(-1.0, 1.0).acos()
}

/// Generate triangle indices for a `nu × nv` grid of vertices (row-major layout).
///
/// Each quad `[i, j]` → 2 triangles: `[tl, bl, tr]` and `[tr, bl, br]`
/// where `tl = i*nv + j`, `tr = i*nv + j+1`, `bl = (i+1)*nv + j`, `br = (i+1)*nv + j+1`.
///
/// Total index count = `6 * (nu - 1) * (nv - 1)`.
pub fn generate_grid_indices(nu: u32, nv: u32) -> Vec<u32> {
    let mut indices = Vec::with_capacity(6 * (nu as usize - 1) * (nv as usize - 1));
    for i in 0..nu - 1 {
        for j in 0..nv - 1 {
            let tl = i * nv + j;
            let tr = i * nv + j + 1;
            let bl = (i + 1) * nv + j;
            let br = (i + 1) * nv + j + 1;
            // Triangle 1: tl, bl, tr
            indices.push(tl);
            indices.push(bl);
            indices.push(tr);
            // Triangle 2: tr, bl, br
            indices.push(tr);
            indices.push(bl);
            indices.push(br);
        }
    }
    indices
}

#[cfg(test)]
mod tests {
    use super::*;
    use ranim_core::{
        anchor::Aabb,
        color::palette::css,
        glam::DVec3,
        traits::{Alignable, Empty},
    };

    #[test]
    fn test_generate_grid_indices_2x2() {
        // 2×2 grid → 1 quad → 2 triangles → 6 indices
        let indices = generate_grid_indices(2, 2);
        assert_eq!(indices.len(), 6);
        // Vertices: 0=tl, 1=tr, 2=bl, 3=br
        assert_eq!(indices, vec![0, 2, 1, 1, 2, 3]);
    }

    #[test]
    fn test_generate_grid_indices_3x3() {
        // 3×3 grid → 4 quads → 8 triangles → 24 indices
        let indices = generate_grid_indices(3, 3);
        assert_eq!(indices.len(), 24);
    }

    #[test]
    fn test_generate_grid_indices_count() {
        let nu = 10;
        let nv = 5;
        let indices = generate_grid_indices(nu, nv);
        assert_eq!(indices.len(), 6 * (nu as usize - 1) * (nv as usize - 1));
    }

    #[test]
    fn test_mesh_item_alignable() {
        let mut mesh1 = MeshItem::from_indexed_vertices(
            vec![DVec3::new(0.0, 0.0, 0.0), DVec3::new(1.0, 0.0, 0.0)],
            vec![0, 1, 2],
        );

        let mut mesh2 = MeshItem::from_indexed_vertices(
            vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(0.0, 1.0, 0.0),
                DVec3::new(1.0, 1.0, 0.0),
            ],
            vec![0, 1, 2, 1, 3, 2],
        );

        // Initially not aligned
        assert!(!mesh1.is_aligned(&mesh2));

        // Align them
        mesh1.align_with(&mut mesh2);

        // Now they should be aligned
        assert!(mesh1.is_aligned(&mesh2));

        // All vertex arrays should have same length
        assert_eq!(mesh1.points.len(), 4);
        assert_eq!(mesh2.points.len(), 4);
        assert_eq!(mesh1.vertex_colors.len(), 4);
        assert_eq!(mesh2.vertex_colors.len(), 4);
        assert_eq!(mesh1.vertex_normals.len(), 4);
        assert_eq!(mesh2.vertex_normals.len(), 4);

        // mesh1's new points should be last point repeated (from PointVec::align_with)
        assert_eq!(mesh1.points[2], DVec3::new(1.0, 0.0, 0.0));
        assert_eq!(mesh1.points[3], DVec3::new(1.0, 0.0, 0.0));

        // mesh2's points should remain unchanged (it was already longer)
        assert_eq!(mesh2.points[0], DVec3::new(0.0, 0.0, 0.0));
        assert_eq!(mesh2.points[3], DVec3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn test_mesh_item_interpolate() {
        use ranim_core::traits::Interpolatable;

        let mut mesh1 = MeshItem::from_indexed_vertices(
            vec![DVec3::new(0.0, 0.0, 0.0), DVec3::new(1.0, 0.0, 0.0)],
            vec![0, 1, 2],
        )
        .with_color(css::RED.with_alpha(1.0));

        let mut mesh2 = MeshItem::from_indexed_vertices(
            vec![DVec3::new(2.0, 0.0, 0.0), DVec3::new(3.0, 0.0, 0.0)],
            vec![0, 1, 3],
        )
        .with_color(css::GREEN.with_alpha(1.0));

        // Align first
        mesh1.align_with(&mut mesh2);

        // Interpolate at t = 0.5
        let interpolated = mesh1.lerp(&mesh2, 0.5);

        // Points should be halfway between
        assert_eq!(interpolated.points[0], DVec3::new(1.0, 0.0, 0.0));
        assert_eq!(interpolated.points[1], DVec3::new(2.0, 0.0, 0.0));

        // triangle_indices should be from mesh2 (since t >= 0.5)
        assert_eq!(interpolated.triangle_indices, vec![0, 1, 3]);
    }

    #[test]
    fn test_mesh_item_aabb() {
        use ranim_core::glam::dvec3;

        let mesh = MeshItem::from_indexed_vertices(
            vec![
                DVec3::new(-1.0, -1.0, -1.0),
                DVec3::new(1.0, -1.0, -1.0),
                DVec3::new(1.0, 1.0, -1.0),
                DVec3::new(-1.0, 1.0, 1.0),
            ],
            vec![0, 1, 2],
        );

        let [min, max] = mesh.aabb();
        assert_eq!(min, dvec3(-1.0, -1.0, -1.0));
        assert_eq!(max, dvec3(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_mesh_item_empty() {
        let mesh = MeshItem::empty();
        assert_eq!(mesh.points.len(), 0);
        assert_eq!(mesh.triangle_indices.len(), 0);
        assert_eq!(mesh.vertex_colors.len(), 0);
        assert_eq!(mesh.vertex_normals.len(), 0);
    }
}
