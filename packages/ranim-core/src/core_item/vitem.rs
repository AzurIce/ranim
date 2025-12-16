use color::{AlphaColor, Srgb};
use glam::{Vec4, vec4};

use crate::{
    Extract,
    components::{rgba::Rgba, width::Width},
    core_item::{CoreItem, vitem_2d::VItem2d},
    traits::FillColor,
};

/// Default vitem stroke width
pub const DEFAULT_STROKE_WIDTH: f32 = 0.02;

#[derive(Debug, Clone, PartialEq)]
/// A primitive for rendering a vitem.
pub struct VItemPrimitive {
    /// Points 2d.
    pub points2d: Vec<Vec4>,
    /// Fill rgbas, see [`Rgba`].
    pub fill_rgbas: Vec<Rgba>,
    /// Stroke rgbs, see [`Rgba`].
    pub stroke_rgbas: Vec<Rgba>,
    /// Stroke widths, see [`Width`].
    pub stroke_widths: Vec<Width>,
}

impl From<VItem2d> for VItemPrimitive {
    fn from(value: VItem2d) -> Self {
        Self {
            points2d: value
                .points2d
                .into_iter()
                .map(|p| {
                    let r = value.origin + value.basis.0 * p.x + value.basis.1 * p.y;
                    vec4(r.x, r.y, r.z, p.z)
                })
                .collect(),
            fill_rgbas: value.fill_rgbas,
            stroke_rgbas: value.stroke_rgbas,
            stroke_widths: value.stroke_widths,
        }
    }
}

impl Default for VItemPrimitive {
    fn default() -> Self {
        Self {
            points2d: vec![Vec4::ZERO; 3],
            stroke_widths: vec![Width::default(); 2],
            stroke_rgbas: vec![Rgba::default(); 2],
            fill_rgbas: vec![Rgba::default(); 2],
        }
    }
}

impl Extract for VItemPrimitive {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        buf.push(CoreItem::VItemPrimitive(self.clone()));
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
