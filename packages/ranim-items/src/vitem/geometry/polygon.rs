use std::f64::consts::{PI, TAU};

use color::{AlphaColor, Srgb};
use itertools::Itertools;
use ranim_core::{
    Extract,
    anchor::Aabb,
    color,
    core_item::CoreItem,
    glam::{DVec2, DVec3},
    traits::{Alignable, ApplyTransform, FillColor, Opacity, StrokeColor, StrokeWidth, With},
};

use crate::vitem::{DEFAULT_STROKE_WIDTH, VItem, geometry::Circle};

/// A square centered at the origin in the XY plane.
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
#[allow(missing_docs)]
pub struct Square {
    pub size: f64,
    pub stroke_rgba: AlphaColor<Srgb>,
    pub stroke_width: f32,
    pub fill_rgba: AlphaColor<Srgb>,
}

#[allow(missing_docs)]
impl Square {
    pub fn new(size: f64) -> Self {
        Self {
            size,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }

    /// Scales the intrinsic square size.
    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.size *= scale;
        self
    }
}

impl Aabb for Square {
    fn aabb(&self) -> [DVec3; 2] {
        let h = self.size / 2.0;
        [DVec3::new(-h, -h, 0.0), DVec3::new(h, h, 0.0)].aabb()
    }
}

impl Alignable for Square {
    fn is_aligned(&self, _other: &Self) -> bool {
        true
    }
    fn align_with(&mut self, _other: &mut Self) {}
}

impl Opacity for Square {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgba = self.stroke_rgba.with_alpha(opacity);
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self
    }
}

impl StrokeColor for Square {
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

impl FillColor for Square {
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

impl From<Square> for Rectangle {
    fn from(value: Square) -> Self {
        let Square {
            size,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        } = value;
        Rectangle {
            size: DVec2::splat(size),
            stroke_rgba,
            stroke_width,
            fill_rgba,
        }
    }
}

impl From<Square> for RegularPolygon {
    fn from(value: Square) -> Self {
        RegularPolygon::new(4, value.size / 2.0 * 2.0f64.sqrt()).with(|x| {
            x.stroke_rgba = value.stroke_rgba;
            x.stroke_width = value.stroke_width;
            x.fill_rgba = value.fill_rgba;
        })
    }
}

impl From<Square> for Polygon {
    fn from(value: Square) -> Self {
        Rectangle::from(value).into()
    }
}
impl From<Square> for VItem {
    fn from(value: Square) -> Self {
        Rectangle::from(value).into()
    }
}
impl Extract for Square {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        VItem::from(self.clone()).extract_into(buf);
    }
}

/// A rectangle centered at the origin in the XY plane.
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
#[allow(missing_docs)]
pub struct Rectangle {
    pub size: DVec2,
    pub stroke_rgba: AlphaColor<Srgb>,
    pub stroke_width: f32,
    pub fill_rgba: AlphaColor<Srgb>,
}

#[allow(missing_docs)]
impl Rectangle {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            size: DVec2::new(width, height),
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
    pub fn width(&self) -> f64 {
        self.size.x.abs()
    }
    pub fn height(&self) -> f64 {
        self.size.y.abs()
    }
    /// Edits the intrinsic dimensions along the canonical X/Y axes.
    pub fn scale_axes(&mut self, scale: DVec2) -> &mut Self {
        self.size *= scale;
        self
    }
}

impl Aabb for Rectangle {
    fn aabb(&self) -> [DVec3; 2] {
        let h = self.size / 2.0;
        [h.extend(0.0), (-h).extend(0.0)].aabb()
    }
}
impl Alignable for Rectangle {
    fn align_with(&mut self, _other: &mut Self) {}
    fn is_aligned(&self, _other: &Self) -> bool {
        true
    }
}
impl Opacity for Rectangle {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgba = self.stroke_rgba.with_alpha(opacity);
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self
    }
}
impl StrokeColor for Rectangle {
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
impl FillColor for Rectangle {
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

impl From<Rectangle> for Polygon {
    fn from(value: Rectangle) -> Self {
        let h = value.size / 2.0;
        Self {
            points: vec![
                DVec3::new(-h.x, -h.y, 0.0),
                DVec3::new(h.x, -h.y, 0.0),
                DVec3::new(h.x, h.y, 0.0),
                DVec3::new(-h.x, h.y, 0.0),
            ],
            stroke_rgba: value.stroke_rgba,
            stroke_width: value.stroke_width,
            fill_rgba: value.fill_rgba,
        }
    }
}
impl From<Rectangle> for VItem {
    fn from(value: Rectangle) -> Self {
        Polygon::from(value).into()
    }
}
impl Extract for Rectangle {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        VItem::from(self.clone()).extract_into(buf);
    }
}

/// A polygon with uniform stroke and fill.
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
#[allow(missing_docs)]
pub struct Polygon {
    pub points: Vec<DVec3>,
    pub stroke_rgba: AlphaColor<Srgb>,
    pub stroke_width: f32,
    pub fill_rgba: AlphaColor<Srgb>,
}
#[allow(missing_docs)]
impl Polygon {
    pub fn new(points: Vec<DVec3>) -> Self {
        Self {
            points,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
}
impl Aabb for Polygon {
    fn aabb(&self) -> [DVec3; 2] {
        self.points.aabb()
    }
}
impl<G: Into<ranim_core::glam::DAffine3>> ApplyTransform<G> for Polygon {
    fn apply(&mut self, transform: G) -> &mut Self {
        self.points.apply(transform.into());
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
        self.points
            .resize(other.points.len(), *self.points.last().unwrap());
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
impl From<Polygon> for VItem {
    fn from(value: Polygon) -> Self {
        let Polygon {
            mut points,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        } = value;
        assert!(points.len() > 2);
        points.push(points[0]);
        let handles = points
            .iter()
            .tuple_windows()
            .map(|(&a, &b)| 0.5 * (a + b))
            .collect::<Vec<_>>();
        let vpoints = points.into_iter().interleave(handles).collect();
        VItem::from_vpoints(vpoints).with(|v| {
            v.set_fill_color(fill_rgba)
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

/// A regular polygon centered at the origin in the XY plane.
#[derive(Debug, Clone, ranim_macros::Interpolatable)]
#[allow(missing_docs)]
pub struct RegularPolygon {
    pub sides: usize,
    pub radius: f64,
    pub stroke_rgba: AlphaColor<Srgb>,
    pub stroke_width: f32,
    pub fill_rgba: AlphaColor<Srgb>,
}
impl Alignable for RegularPolygon {
    fn is_aligned(&self, _other: &Self) -> bool {
        true
    }
    fn align_with(&mut self, _other: &mut Self) {}
}
#[allow(missing_docs)]
impl RegularPolygon {
    pub fn new(sides: usize, radius: f64) -> Self {
        assert!(sides >= 3);
        Self {
            sides,
            radius,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
    pub fn points(&self) -> Vec<DVec3> {
        (0..self.sides)
            .map(|i| {
                let a = TAU * i as f64 / self.sides as f64;
                DVec3::new(a.cos() * self.radius, a.sin() * self.radius, 0.0)
            })
            .collect()
    }
    pub fn outer_circle(&self) -> Circle {
        Circle::new(self.radius)
    }
    pub fn inner_circle(&self) -> Circle {
        Circle::new(self.radius * (PI / self.sides as f64).cos())
    }
}
impl Aabb for RegularPolygon {
    fn aabb(&self) -> [DVec3; 2] {
        self.points().aabb()
    }
}
impl Opacity for RegularPolygon {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self.stroke_rgba = self.stroke_rgba.with_alpha(opacity);
        self
    }
}
impl FillColor for RegularPolygon {
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
impl StrokeColor for RegularPolygon {
    fn stroke_color(&self) -> AlphaColor<Srgb> {
        self.stroke_rgba
    }
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgba = self.stroke_rgba.with_alpha(opacity);
        self
    }
    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.stroke_rgba = color;
        self
    }
}
impl From<RegularPolygon> for Polygon {
    fn from(value: RegularPolygon) -> Self {
        Polygon::new(value.points()).with(|x| {
            x.fill_rgba = value.fill_rgba;
            x.stroke_rgba = value.stroke_rgba;
            x.stroke_width = value.stroke_width;
        })
    }
}
impl Extract for RegularPolygon {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        Polygon::from(self.clone()).extract_into(buf);
    }
}
