use color::{AlphaColor, Srgb};
use glam::Vec3;

use crate::{
    Extract,
    components::{rgba::Rgba, width::Width},
    core_item::CoreItem,
    traits::FillColor,
};

/// Default vitem stroke width
pub const DEFAULT_STROKE_WIDTH: f32 = 2.0;

#[derive(Debug, Clone, PartialEq)]
/// A primitive for rendering a vitem.
pub struct VItem2d {
    /// The base point of the item, a.k.a. the origin of the item's local coordinate system.
    pub origin: Vec3,
    /// The basis vectors of the item's local coordinate system. Normalized.
    pub basis: (Vec3, Vec3),
    /// The points of the item in the item's local coordinate system.
    /// (x, y, is_closed)
    pub points2d: Vec<Vec3>,
    /// Fill rgbas, see [`Rgba`].
    pub fill_rgbas: Vec<Rgba>,
    /// Stroke rgbs, see [`Rgba`].
    pub stroke_rgbas: Vec<Rgba>,
    /// Stroke widths, see [`Width`].
    pub stroke_widths: Vec<Width>,
}

impl Default for VItem2d {
    fn default() -> Self {
        Self {
            origin: Vec3::ZERO,
            basis: (Vec3::X, Vec3::Y),
            points2d: vec![Vec3::ZERO; 3],
            stroke_widths: vec![Width::default(); 2],
            stroke_rgbas: vec![Rgba::default(); 2],
            fill_rgbas: vec![Rgba::default(); 2],
        }
    }
}

impl Extract for VItem2d {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        buf.push(CoreItem::VItem2D(self.clone()));
    }
}

impl FillColor for VItem2d {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        let Rgba(rgba) = self.fill_rgbas[0];
        AlphaColor::new([rgba.x, rgba.y, rgba.z, rgba.w])
    }
    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.fill_rgbas.fill(color.into());
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgbas
            .iter_mut()
            .for_each(|rgba| rgba.0.w = opacity);
        self
    }
}
