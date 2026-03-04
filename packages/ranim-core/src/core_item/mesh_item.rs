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
    /// The fill color
    pub fill_rgba: Rgba,
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
            fill_rgba: self.fill_rgba.lerp(&target.fill_rgba, t),
        }
    }
}

impl Default for MeshItem {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            triangle_indices: Vec::new(),
            transform: Mat4::IDENTITY,
            fill_rgba: Rgba::default(),
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
        let Rgba(rgba) = self.fill_rgba;
        AlphaColor::new([rgba.x, rgba.y, rgba.z, rgba.w])
    }

    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.fill_rgba = color.into();
        self
    }

    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgba.0.w = opacity;
        self
    }
}
