use glam::Vec4;

use crate::components::{rgba::Rgba, width::Width};

pub const DEFAULT_STROKE_WIDTH: f32 = 0.02;

#[derive(Clone)]
/// A primitive for rendering a vitem.
pub struct VItemPrimitive {
    pub points2d: Vec<Vec4>,
    pub fill_rgbas: Vec<Rgba>,
    pub stroke_rgbas: Vec<Rgba>,
    pub stroke_widths: Vec<Width>,
}
