use ranim_core::{Extract, color, core_item::CoreItem, glam::{self, DVec2}, traits::Anchor};

use color::{AlphaColor, Srgb};
use glam::{DVec3, dvec3};
use itertools::Itertools;

use crate::vitem::{DEFAULT_STROKE_WIDTH, Proj};
use ranim_core::traits::{
    Alignable, BoundingBox, FillColor, Opacity, Rotate, Scale, Shift, StrokeColor, StrokeWidth,
    With,
};

use crate::vitem::VItem;

// MARK: ### Square ###
/// A Square
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Square {
    /// Projection
    pub proj: Proj,
    /// Center
    pub center: DVec3,
    /// Size
    pub size: f64,

    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
    /// Fill rgba
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Square {
    /// Constructor
    pub fn new(size: f64) -> Self {
        Self {
            proj: Proj::default(),
            center: dvec3(0.0, 0.0, 0.0),
            size,

            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
    /// Scale the square by the given scale, with the given anchor as the center.
    ///
    /// Note that this accepts a `f64` scale dispite of [`Scale`]'s `DVec3`,
    /// because this keeps the square a square.
    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.scale_by_anchor(scale, Anchor::CENTER)
    }
    /// Scale the square by the given scale, with the given anchor as the center.
    ///
    /// Note that this accepts a `f64` scale dispite of [`Scale`]'s `DVec3`,
    /// because this keeps the square a square.
    pub fn scale_by_anchor(&mut self, scale: f64, anchor: Anchor) -> &mut Self {
        let anchor = Anchor::Point(match anchor {
            Anchor::Point(point) => point,
            Anchor::Edge(edge) => self.get_bounding_box_point(edge),
        });
        self.size *= scale;
        self.center.scale_by_anchor(DVec3::splat(scale), anchor);
        self
    }
}

// MARK: Traits impl
impl BoundingBox for Square {
    fn get_min_max(&self) -> [DVec3; 2] {
        let (u, v) = self.proj.basis();
        [
            self.center + self.size / 2.0 * (u + v),
            self.center - self.size / 2.0 * (u + v),
        ]
        .get_min_max()
    }
}

impl Shift for Square {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.center.shift(shift);
        self
    }
}

impl Rotate for Square {
    fn rotate_by_anchor(&mut self, angle: f64, axis: DVec3, anchor: Anchor) -> &mut Self {
        let anchor = Anchor::Point(match anchor {
            Anchor::Point(point) => point,
            Anchor::Edge(edge) => self.get_bounding_box_point(edge),
        });
        self.center.rotate_by_anchor(angle, axis, anchor);
        self.proj.rotate(angle, axis);
        self
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

impl Extract for Square {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        VItem::from(self.clone()).extract_into(buf);
    }
}

// MARK: Conversions
impl From<Square> for Rectangle {
    fn from(value: Square) -> Self {
        let Square {
            proj,
            center,
            size: width,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        } = value;
        let (u, v) = proj.basis();
        let p1 = center - width / 2.0 * u + width / 2.0 * v;
        let p2 = center + width / 2.0 * u - width / 2.0 * v;
        Rectangle {
            proj,
            p1,
            p2,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        }
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

// MARK: ### Rectangle ###
/// Rectangle
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Rectangle {
    /// Projection info
    pub proj: Proj,
    /// Bottom left corner (minimum)
    pub p1: DVec3,
    /// Top right corner (maximum)
    pub p2: DVec3,

    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
    /// Fill rgba
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Default for Rectangle {
    fn default() -> Self {
        Self {
            proj: Proj::default(),
            p1: dvec3(0.0, 0.0, 0.0),
            p2: dvec3(1.0, 1.0, 0.0),
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
}

impl Rectangle {
    /// Constructor
    pub fn new(width: f64, height: f64) -> Self {
        let half_width = width / 2.0;
        let half_height = height / 2.0;
        Self {
            p1: dvec3(-half_width, -half_height, 0.0),
            p2: dvec3(half_width, half_height, 0.0),
            ..Default::default()
        }
    }
    /// Construct a rectangle from the bottom-left point (minimum) and size.
    pub fn from_min_size(p_min: DVec3, size: DVec2) -> Self {
        let p1 = p_min;
        let p2 = p1 + dvec3(size.x, size.y, 0.);
        Self {
            p1,
            p2,
            ..Default::default()
        }
    }
    /// Construct a rectangle from two corner points
    pub fn from_two_points(p1: DVec3, p2: DVec3) -> Self {
        let (x1, x2) = if p1.x < p2.x { (p1.x, p2.x) } else { (p2.x, p1.x) };
        let (y1, y2) = if p1.y < p2.y { (p1.y, p2.y) } else { (p2.y, p1.y) };
        let z = p1.z;
        let p1 = dvec3(x1, y1, z);
        let p2 = dvec3(x2, y2, z);
        Self {
            p1,
            p2,
            ..Default::default()
        }
    }
    /// Width
    pub fn width(&self) -> f64 {
        (self.p2 - self.p1).dot(self.proj.basis_u()).abs()
    }
    /// Height
    pub fn height(&self) -> f64 {
        (self.p2 - self.p1).dot(self.proj.basis_v()).abs()
    }
}

// MARK: Traits impl
impl BoundingBox for Rectangle {
    fn get_min_max(&self) -> [DVec3; 2] {
        [self.p1, self.p2].get_min_max()
    }
}

impl Shift for Rectangle {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.p1.shift(shift);
        self.p2.shift(shift);
        self
    }
}

impl Rotate for Rectangle {
    fn rotate_by_anchor(&mut self, angle: f64, axis: DVec3, anchor: Anchor) -> &mut Self {
        let anchor = Anchor::Point(anchor.get_pos(self));
        self.p1.rotate_by_anchor(angle, axis, anchor);
        self.p2.rotate_by_anchor(angle, axis, anchor);
        self.proj.rotate(angle, axis);
        self
    }
}

impl Scale for Rectangle {
    fn scale_by_anchor(&mut self, scale: DVec3, anchor: Anchor) -> &mut Self {
        let anchor = Anchor::Point(anchor.get_pos(self));
        self.p1.scale_by_anchor(scale, anchor);
        self.p2.scale_by_anchor(scale, anchor);
        self
    }
}

impl Opacity for Rectangle {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke_rgba = self.stroke_rgba.with_alpha(opacity);
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self
    }
}

impl Alignable for Rectangle {
    fn align_with(&mut self, _other: &mut Self) {}
    fn is_aligned(&self, _other: &Self) -> bool {
        true
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

// MARK: Conversions
impl From<Rectangle> for Polygon {
    fn from(value: Rectangle) -> Self {
        let points = vec![
            value.p1,
            value.p1 + value.proj.basis_v() * value.height(),
            value.p2,
            value.p2 + value.proj.basis_v() * value.height(),
        ];
        Polygon {
            proj: value.proj,
            points,
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

// MARK: ### Polygon ###
/// A Polygon with uniform stroke and fill
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Polygon {
    /// Projection info
    pub proj: Proj,
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
            proj: Proj::default(),
            points,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
}

// MARK: Traits impl
impl BoundingBox for Polygon {
    fn get_min_max(&self) -> [DVec3; 2] {
        self.points.get_min_max()
    }
}

impl Shift for Polygon {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.points.shift(shift);
        self
    }
}

impl Rotate for Polygon {
    fn rotate_by_anchor(&mut self, angle: f64, axis: DVec3, anchor: Anchor) -> &mut Self {
        self.points.rotate_by_anchor(angle, axis, anchor);
        self
    }
}

impl Scale for Polygon {
    fn scale_by_anchor(&mut self, scale: DVec3, anchor: Anchor) -> &mut Self {
        self.points.scale_by_anchor(scale, anchor);
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
