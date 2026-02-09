use ranim_core::{
    Extract,
    anchor::Aabb,
    color,
    core_item::CoreItem,
    glam,
    traits::{Rotate, RotateExt, Scale, Shift},
};

use color::{AlphaColor, Srgb};
use glam::DVec3;
use itertools::Itertools;

use crate::vitem::{DEFAULT_STROKE_WIDTH, ProjectionPlane, VItem};
use ranim_core::traits::{Alignable, FillColor, Opacity, ScaleExt, StrokeColor, StrokeWidth, With};

// MARK: ### Polygon ###
/// A Polygon with uniform stroke and fill
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Polygon {
    /// Projection info
    pub proj: ProjectionPlane,
    /// Corner points
    pub points: Vec<DVec3>,
    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
    /// Fill rgba
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Polygon {
    /// Constructor
    pub fn new(points: Vec<DVec3>) -> Self {
        Self {
            proj: ProjectionPlane::default(),
            points,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
}

// MARK: Traits impl
impl Aabb for Polygon {
    fn aabb(&self) -> [DVec3; 2] {
        self.points.aabb()
    }
}

impl Shift for Polygon {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.points.shift(shift);
        self
    }
}

impl Rotate for Polygon {
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self {
        self.points.rotate_at(angle, axis, point);
        self
    }
}

impl Scale for Polygon {
    fn scale_at_point(&mut self, scale: DVec3, point: DVec3) -> &mut Self {
        self.points.scale_at(scale, point);
        self
    }
}

impl Alignable for Polygon {
    fn is_aligned(&self, other: &Self) -> bool {
        self.points.len() == other.points.len()
    }
    fn align_with(&mut self, other: &mut Self) {
        if self.points.len() > other.points.len() {
            return other.align_with(self);
        }
        // TODO: find a better algo to minimize the distance
        self.points
            .resize(other.points.len(), self.points.last().cloned().unwrap());
    }
}

impl Opacity for Polygon {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self.stroke_rgba = self.stroke_rgba.with_alpha(opacity);
        self
    }
}

impl StrokeColor for Polygon {
    fn stroke_color(&self) -> AlphaColor<Srgb> {
        self.stroke_rgba
    }
    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.stroke_rgba = color;
        self
    }
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgba = self.stroke_rgba.with_alpha(opacity);
        self
    }
}

impl FillColor for Polygon {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.fill_rgba
    }
    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.fill_rgba = color;
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self
    }
}

// MARK: Conversions
impl From<Polygon> for VItem {
    fn from(value: Polygon) -> Self {
        let Polygon {
            mut points,
            stroke_rgba,
            stroke_width,
            fill_rgba,
            proj,
            ..
        } = value;
        assert!(points.len() > 2);

        // Close the polygon
        points.push(points[0]);

        let anchors = points;
        let handles = anchors
            .iter()
            .tuple_windows()
            .map(|(&a, &b)| 0.5 * (a + b))
            .collect::<Vec<_>>();

        // Interleave anchors and handles
        let vpoints = anchors.into_iter().interleave(handles).collect::<Vec<_>>();
        VItem::from_vpoints(vpoints).with_proj(proj).with(|vitem| {
            vitem
                .set_fill_color(fill_rgba)
                .set_stroke_color(stroke_rgba)
                .set_stroke_width(stroke_width);
        })
    }
}

impl Extract for Polygon {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        VItem::from(self.clone()).extract_into(buf);
    }
}
