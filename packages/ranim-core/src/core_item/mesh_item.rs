use glam::{DVec3, Mat4, Vec3};

use crate::{
    CoreItem, Extract,
    anchor::Aabb,
    components::{PointVec, rgba::Rgba},
    traits::{
        Alignable, Empty, FillColor, Interpolatable, Opacity, RotateTransform, ScaleTransform,
        ShiftTransform,
    },
};
use color::{AlphaColor, Srgb};

/// A mesh item with per-vertex data.
///
/// This struct uses [`PointVec`] to wrap vertex data, enabling alignment and
/// interpolation for animations.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshItem {
    /// The vertices of the mesh
    pub points: PointVec<Vec3>,
    /// The triangle indices
    pub triangle_indices: Vec<u32>,
    /// The transform matrix
    pub transform: Mat4,
    /// Per-vertex colors
    pub vertex_colors: PointVec<Rgba>,
    /// Per-vertex normals for smooth shading.
    /// All-zero (or empty) → shader falls back to flat shading via `dpdx`/`dpdy`.
    pub vertex_normals: PointVec<Vec3>,
}

impl MeshItem {
    /// Create a MeshItem from vertices only (no indices, suitable for point clouds).
    pub fn from_vertices(points: Vec<Vec3>) -> Self {
        let len = points.len();
        Self {
            points: points.into(),
            triangle_indices: Vec::new(),
            transform: Mat4::IDENTITY,
            vertex_colors: vec![Rgba::default(); len].into(),
            vertex_normals: vec![Vec3::ZERO; len].into(),
        }
    }

    /// Create a MeshItem from vertices and triangle indices.
    pub fn from_indexed_vertices(points: Vec<Vec3>, triangle_indices: Vec<u32>) -> Self {
        let len = points.len();
        Self {
            points: points.into(),
            triangle_indices,
            transform: Mat4::IDENTITY,
            vertex_colors: vec![Rgba::default(); len].into(),
            vertex_normals: vec![Vec3::ZERO; len].into(),
        }
    }

    /// Set the transform matrix.
    pub fn with_transform(mut self, transform: Mat4) -> Self {
        self.transform = transform;
        self
    }

    /// Set all vertex colors to the same value.
    pub fn with_color(mut self, color: AlphaColor<Srgb>) -> Self {
        let rgba: Rgba = color.into();
        self.vertex_colors = vec![rgba; self.points.len()].into();
        self
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
            transform: self.transform.lerp(&target.transform, t),
            vertex_colors: self.vertex_colors.lerp(&target.vertex_colors, t),
            vertex_normals: self.vertex_normals.lerp(&target.vertex_normals, t),
        }
    }
}

impl Default for MeshItem {
    fn default() -> Self {
        Self {
            points: vec![Vec3::ZERO; 3].into(),
            triangle_indices: vec![0, 1, 2],
            transform: Mat4::IDENTITY,
            vertex_colors: vec![Rgba::default(); 3].into(),
            vertex_normals: vec![Vec3::ZERO; 3].into(),
        }
    }
}

impl Extract for MeshItem {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        buf.push(CoreItem::MeshItem(self.clone()));
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

        let transform = self.transform.as_dmat4();
        let transformed_points: Vec<DVec3> = self
            .points
            .iter()
            .map(|&p| transform.transform_point3(p.as_dvec3()))
            .collect();

        let mut min = transformed_points[0];
        let mut max = transformed_points[0];

        for &p in &transformed_points[1..] {
            min = min.min(p);
            max = max.max(p);
        }

        [min, max]
    }
}

impl ShiftTransform for MeshItem {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        let translation = Mat4::from_translation(offset.as_vec3());
        self.transform = translation * self.transform;
        self
    }
}

impl RotateTransform for MeshItem {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        let rotation = Mat4::from_axis_angle(axis.as_vec3().normalize(), angle as f32);
        self.transform = rotation * self.transform;
        self
    }
}

impl ScaleTransform for MeshItem {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        let scale_mat = Mat4::from_scale(scale.as_vec3());
        self.transform = scale_mat * self.transform;
        self
    }
}

impl Empty for MeshItem {
    fn empty() -> Self {
        Self {
            points: Vec::new().into(),
            triangle_indices: Vec::new(),
            transform: Mat4::IDENTITY,
            vertex_colors: Vec::new().into(),
            vertex_normals: Vec::new().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        color::palette::css,
        glam::{Mat4, Vec3, dvec3},
        traits::{Alignable, Empty, RotateTransform, ScaleTransform, ShiftTransform},
    };
    use std::f64::consts::PI;

    #[test]
    fn mesh_item_alignable() {
        let mut mesh1 = MeshItem::from_indexed_vertices(
            vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)],
            vec![0, 1, 2],
        );

        let mut mesh2 = MeshItem::from_indexed_vertices(
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
            ],
            vec![0, 1, 2, 1, 3, 2],
        );

        assert!(!mesh1.is_aligned(&mesh2));

        mesh1.align_with(&mut mesh2);

        assert!(mesh1.is_aligned(&mesh2));
        assert_eq!(mesh1.points.len(), 4);
        assert_eq!(mesh2.points.len(), 4);
        assert_eq!(mesh1.vertex_colors.len(), 4);
        assert_eq!(mesh2.vertex_colors.len(), 4);
        assert_eq!(mesh1.vertex_normals.len(), 4);
        assert_eq!(mesh2.vertex_normals.len(), 4);
        assert_eq!(mesh1.points[2], Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(mesh1.points[3], Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(mesh2.points[0], Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(mesh2.points[3], Vec3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn mesh_item_interpolate() {
        let mut mesh1 = MeshItem::from_indexed_vertices(
            vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)],
            vec![0, 1, 2],
        )
        .with_color(css::RED.with_alpha(1.0));

        let mut mesh2 = MeshItem::from_indexed_vertices(
            vec![Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 0.0, 0.0)],
            vec![0, 1, 3],
        )
        .with_color(css::GREEN.with_alpha(1.0))
        .with_transform(Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0)));

        mesh1.align_with(&mut mesh2);

        let interpolated = mesh1.lerp(&mesh2, 0.5);

        assert_eq!(interpolated.points[0], Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(interpolated.points[1], Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(interpolated.triangle_indices, vec![0, 1, 3]);
        assert_eq!(
            interpolated.transform,
            Mat4::from_translation(Vec3::new(0.5, 0.0, 0.0))
        );
    }

    #[test]
    fn mesh_item_aabb() {
        let mesh = MeshItem::from_indexed_vertices(
            vec![
                Vec3::new(-1.0, -1.0, -1.0),
                Vec3::new(1.0, -1.0, -1.0),
                Vec3::new(1.0, 1.0, -1.0),
                Vec3::new(-1.0, 1.0, 1.0),
            ],
            vec![0, 1, 2],
        );

        let [min, max] = mesh.aabb();
        assert_eq!(min, dvec3(-1.0, -1.0, -1.0));
        assert_eq!(max, dvec3(1.0, 1.0, 1.0));
    }

    #[test]
    fn mesh_item_shift() {
        let mut mesh = MeshItem::from_indexed_vertices(
            vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)],
            vec![0, 1],
        );

        mesh.shift(dvec3(1.0, 2.0, 3.0));

        let [min, _max] = mesh.aabb();
        assert!((min.x - 1.0).abs() < 1e-5);
        assert!((min.y - 2.0).abs() < 1e-5);
        assert!((min.z - 3.0).abs() < 1e-5);
    }

    #[test]
    fn mesh_item_scale() {
        let mut mesh = MeshItem::from_indexed_vertices(
            vec![Vec3::new(1.0, 1.0, 1.0), Vec3::new(2.0, 2.0, 2.0)],
            vec![0, 1],
        );

        mesh.scale(dvec3(2.0, 2.0, 2.0));

        let [min, max] = mesh.aabb();
        assert!((min.x - 2.0).abs() < 1e-5);
        assert!((max.x - 4.0).abs() < 1e-5);
    }

    #[test]
    fn mesh_item_rotate() {
        let mut mesh = MeshItem::from_indexed_vertices(vec![Vec3::new(1.0, 0.0, 0.0)], vec![]);

        mesh.rotate_on_axis(dvec3(0.0, 0.0, 1.0), PI / 2.0);

        let [min, _max] = mesh.aabb();
        assert!(min.x.abs() < 1e-5);
        assert!((min.y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn mesh_item_empty() {
        let mesh = MeshItem::empty();
        assert_eq!(mesh.points.len(), 0);
        assert_eq!(mesh.triangle_indices.len(), 0);
        assert_eq!(mesh.vertex_colors.len(), 0);
        assert_eq!(mesh.vertex_normals.len(), 0);
    }
}
