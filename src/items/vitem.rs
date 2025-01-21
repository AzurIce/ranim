use std::ops::Deref;

use glam::{vec3, vec4, Vec4};

use crate::{
    components::{rgba::Rgba, vpoint::VPoint, width::Width, ComponentData},
    rabject::Rabject,
    render::primitives::vitem::VItemPrimitive,
};

use super::Extract;

pub struct VItem {
    pub vpoints: ComponentData<VPoint>,
    pub stroke_widths: ComponentData<Width>,
    pub stroke_rgbas: ComponentData<Rgba>,
    pub fill_rgbas: ComponentData<Rgba>,
}

impl VItem {
    pub fn square() -> Self {
        let vpoints = vec![
            VPoint::new(vec3(-100.0, -100.0, 0.0)),
            VPoint::new(vec3(-100.0, 0.0, 0.0)),
            VPoint::new(vec3(-100.0, 100.0, 0.0)),
            // VPoint::new(vec3(-100.0, 100.0, 0.0)),
            VPoint::new(vec3(0.0, 100.0, 0.0)),
            VPoint::new(vec3(100.0, 100.0, 0.0)),
            VPoint::new(vec3(100.0, 0.0, 0.0)),
            VPoint::new(vec3(100.0, -100.0, 0.0)),
            VPoint::new(vec3(0.0, -100.0, 0.0)),
            VPoint::new(vec3(-100.0, -100.0, 0.0)),
        ];
        let stroke_widths = vec![Width(1.0); vpoints.len()];
        let stroke_rgbas = vec![Rgba(vec4(1.0, 0.0, 0.0, 1.0)); vpoints.len()];
        let fill_rgbas = vec![Rgba(vec4(0.0, 1.0, 0.0, 0.5)); vpoints.len()];
        Self {
            vpoints: vpoints.into(),
            stroke_rgbas: stroke_rgbas.into(),
            stroke_widths: stroke_widths.into(),
            fill_rgbas: fill_rgbas.into(),
        }
    }
}

pub struct ExtractedVItem {
    pub points: Vec<Vec4>,
    pub stroke_widths: Vec<f32>,
    pub stroke_rgbas: Vec<Vec4>,
    pub fill_rgbas: Vec<Vec4>,
}

impl Extract for VItem {
    type ExtractData = ExtractedVItem;
    fn extract(&self) -> Self::ExtractData {
        ExtractedVItem {
            points: self
                .vpoints
                .iter()
                .zip(self.vpoints.get_closepath_flags().iter())
                .map(|(p, f)| vec4(p.x, p.y, p.z, if *f { 1.0 } else { 0.0 }))
                .collect(),
            stroke_widths: self.stroke_widths.iter().map(|w| *w.deref()).collect(),
            stroke_rgbas: self.stroke_rgbas.iter().map(|c| *c.deref()).collect(),
            fill_rgbas: self.fill_rgbas.iter().map(|c| *c.deref()).collect(),
        }
    }
}

impl Rabject for VItem {
    type ExtractData = ExtractedVItem;
    type RenderResource = VItemPrimitive;
    fn extract(&self) -> Self::ExtractData {
        Extract::extract(self)
    }
}
