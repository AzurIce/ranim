use color::{AlphaColor, Srgb};
use glam::Vec4;

use crate::{
    components::{rgba::Rgba, width::Width},
    primitives::{Primitive, Primitives},
    traits::FillColor,
};

pub const DEFAULT_STROKE_WIDTH: f32 = 0.02;

#[derive(Debug, Clone, PartialEq)]
/// A primitive for rendering a vitem.
pub struct VItemPrimitive {
    pub points2d: Vec<Vec4>,
    pub fill_rgbas: Vec<Rgba>,
    pub stroke_rgbas: Vec<Rgba>,
    pub stroke_widths: Vec<Width>,
}

impl Primitive for VItemPrimitive {
    fn build_primitives<T: IntoIterator<Item = Self>>(iter: T) -> super::Primitives {
        Primitives::VItemPrimitive(iter.into_iter().collect())
    }
}

impl FillColor for VItemPrimitive {
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
