use glam::{Mat4, Vec3};

use crate::{
    Extract,
    components::rgba::Rgba,
    core_item::CoreItem,
    traits::{FillColor, Interpolatable},
};
use color::{AlphaColor, Srgb};

/// A primitive for rendering a mesh item.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshItem {
    /// The vertices of the mesh
    pub points: Vec<Vec3>,
    /// The triangle indices
    pub triangle_indices: Vec<u32>,
    /// The transform matrix
    pub transform: Mat4,
    /// Optional per-vertex colors. When `Some`, overrides `fill_rgba`.
    pub vertex_colors: Vec<Rgba>,
    /// Per-vertex normals for smooth shading.
    /// All-zero (or empty) → shader falls back to flat shading via `dpdx`/`dpdy`.
    pub vertex_normals: Vec<Vec3>,
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
            points: Vec::new(),
            triangle_indices: Vec::new(),
            transform: Mat4::IDENTITY,
            vertex_colors: Vec::new(),
            vertex_normals: Vec::new(),
        }
    }
}

impl Extract for MeshItem {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        buf.push(CoreItem::MeshItem(self.clone()));
    }
}

impl FillColor for MeshItem {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        let Rgba(rgba) = self.vertex_colors.first().cloned().unwrap_or_default();
        AlphaColor::new([rgba.x, rgba.y, rgba.z, rgba.w])
    }

    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        if let Some(x) = self.vertex_colors.first_mut() {
            *x = color.into();
        }
        self
    }

    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        if let Some(x) = self.vertex_colors.first_mut() {
            x.0.w = opacity;
        }
        self
    }
}
