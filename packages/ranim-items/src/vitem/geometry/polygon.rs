use ranim_core::{Extract, color, core_item::CoreItem, glam, traits::Anchor};

use color::{AlphaColor, Srgb};
use glam::{DVec3, dvec3};
use itertools::Itertools;

use ranim_core::{
    core_item::vitem::DEFAULT_STROKE_WIDTH,
    traits::{
        Alignable, BoundingBox, FillColor, Interpolatable, Opacity, Rotate, Scale, Shift,
        StrokeColor, StrokeWidth, With,
    },
};

use crate::vitem::VItem;

// MARK: ### Square ###
/// A Square
#[derive(Clone, Debug)]
pub struct Square {
    /// Center
    pub center: DVec3,
    /// Size
    pub size: f64,
    /// Up vec
    pub up: DVec3,
    /// Normal vec
    pub normal: DVec3,

    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
    /// Fill rgba
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Interpolatable for Square {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            center: Interpolatable::lerp(&self.center, &target.center, t),
            size: Interpolatable::lerp(&self.size, &target.size, t),
            up: Interpolatable::lerp(&self.up, &target.up, t),
            normal: Interpolatable::lerp(&self.normal, &target.normal, t),
            stroke_rgba: Interpolatable::lerp(&self.stroke_rgba, &target.stroke_rgba, t),
            stroke_width: Interpolatable::lerp(&self.stroke_width, &target.stroke_width, t),
            fill_rgba: Interpolatable::lerp(&self.fill_rgba, &target.fill_rgba, t),
        }
    }
}

impl Square {
    /// Constructor
    pub fn new(size: f64) -> Self {
        Self {
            center: dvec3(0.0, 0.0, 0.0),
            size,
            up: dvec3(0.0, 1.0, 0.0),
            normal: dvec3(0.0, 0.0, 1.0),

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
    fn get_bounding_box(&self) -> [DVec3; 3] {
        let right = -self.normal.cross(self.up).normalize();
        [
            self.center - self.size / 2.0 * right + self.size / 2.0 * self.up,
            self.center + self.size / 2.0 * right - self.size / 2.0 * self.up,
        ]
        .get_bounding_box()
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
        self.up.rotate_by_anchor(angle, axis, Anchor::ORIGIN);
        self.normal.rotate_by_anchor(angle, axis, Anchor::ORIGIN);
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
    fn extract(&self) -> Vec<Self::Target> {
        VItem::from(self.clone()).extract()
    }
}

// MARK: Conversions
impl From<Square> for Rectangle {
    fn from(value: Square) -> Self {
        let Square {
            center,
            size: width,
            up,
            normal,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        } = value;
        let right = up.cross(normal).normalize();
        let p1 = center - width / 2.0 * right + width / 2.0 * up;
        let p2 = center + width / 2.0 * right - width / 2.0 * up;
        Rectangle {
            p1,
            p2,
            up,
            normal,
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
#[derive(Clone, Debug)]
pub struct Rectangle {
    /// Corner 1
    pub p1: DVec3,
    /// Corner 2
    pub p2: DVec3,
    up: DVec3,
    /// Normal vec
    pub normal: DVec3,

    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
    /// Fill rgba
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Interpolatable for Rectangle {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            p1: Interpolatable::lerp(&self.p1, &target.p1, t),
            p2: Interpolatable::lerp(&self.p2, &target.p2, t),
            up: Interpolatable::lerp(&self.up, &target.up, t),
            normal: Interpolatable::lerp(&self.normal, &target.normal, t),
            stroke_rgba: Interpolatable::lerp(&self.stroke_rgba, &target.stroke_rgba, t),
            stroke_width: Interpolatable::lerp(&self.stroke_width, &target.stroke_width, t),
            fill_rgba: Interpolatable::lerp(&self.fill_rgba, &target.fill_rgba, t),
        }
    }
}

impl Rectangle {
    /// Constructor
    pub fn new(width: f64, height: f64) -> Self {
        let half_width = width / 2.0;
        let half_height = height / 2.0;
        Self {
            p1: dvec3(-half_width, half_height, 0.0),
            p2: dvec3(half_width, -half_height, 0.0),
            up: DVec3::Y,
            normal: DVec3::Z,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
    /// Width
    pub fn width(&self) -> f64 {
        let right = self.up.cross(self.normal).normalize();
        (self.p2 - self.p1).dot(right).abs()
    }
    /// Height
    pub fn height(&self) -> f64 {
        (self.p2 - self.p1).dot(self.up).abs()
    }
}

// MARK: Traits impl
impl BoundingBox for Rectangle {
    fn get_bounding_box(&self) -> [DVec3; 3] {
        [self.p1, self.p2].get_bounding_box()
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
        self.up.rotate_by_anchor(angle, axis, Anchor::ORIGIN);
        self.normal.rotate_by_anchor(angle, axis, Anchor::ORIGIN);
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
            value.p1 - value.up * value.height(),
            value.p2,
            value.p2 + value.up * value.height(),
        ];
        Polygon {
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
    fn extract(&self) -> Vec<Self::Target> {
        VItem::from(self.clone()).extract()
    }
}

// MARK: ### Polygon ###
/// A Polygon with uniform stroke and fill
#[derive(Clone, Debug)]
pub struct Polygon {
    /// Corner points
    pub points: Vec<DVec3>,
    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
    /// Fill rgba
    pub fill_rgba: AlphaColor<Srgb>,
    // _need_update: RefCell<bool>,
    // _extract_cache: RefCell<Option<VItem>>,
}

impl Polygon {
    /// Constructor
    pub fn new(points: Vec<DVec3>) -> Self {
        Self {
            points,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
            // _need_update: RefCell::new(true),
            // _extract_cache: RefCell::new(None),
        }
    }
}

// MARK: Traits impl
impl BoundingBox for Polygon {
    fn get_bounding_box(&self) -> [DVec3; 3] {
        self.points.get_bounding_box()
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

impl Interpolatable for Polygon {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            points: self
                .points
                .iter()
                .zip(target.points.iter())
                .map(|(a, b)| a.lerp(b, t))
                .collect(),
            stroke_rgba: Interpolatable::lerp(&self.stroke_rgba, &target.stroke_rgba, t),
            stroke_width: self.stroke_width.lerp(&target.stroke_width, t),
            fill_rgba: Interpolatable::lerp(&self.fill_rgba, &target.fill_rgba, t),
            // _need_update: RefCell::new(true),
            // _extract_cache: RefCell::new(None),
        }
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
        VItem::from_vpoints(vpoints).with(|vitem| {
            vitem
                .set_fill_color(fill_rgba)
                .set_stroke_color(stroke_rgba)
                .set_stroke_width(stroke_width);
        })
    }
}

impl Extract for Polygon {
    type Target = CoreItem;
    fn extract(&self) -> Vec<Self::Target> {
        // trace!("extract");
        // let mut need_update = self._need_update.borrow_mut();
        // let mut cache = self._extract_cache.borrow_mut();
        // if *need_update || cache.is_none() {
        //     trace!("extract: replace vitem");
        //     cache.replace(VItem::from(self.clone()));
        //     *need_update = false;
        // }
        // cache.as_ref().unwrap().extract()
        VItem::from(self.clone()).extract()
    }
}

#[cfg(test)]
mod tests {
    use assert_float_eq::assert_float_absolute_eq;

    use super::*;
    #[test]
    fn test_square() {
        let square = Square::new(2.0).with(|data| {
            data.shift(DVec3::NEG_Y)
                .rotate(std::f64::consts::PI / 2.0, DVec3::X);
        });
        assert_float_absolute_eq!(square.center.distance_squared(DVec3::NEG_Y), 0.0, 1e-10);
        assert_float_absolute_eq!(square.up.distance_squared(DVec3::Z), 0.0, 1e-10);
        assert_float_absolute_eq!(square.normal.distance_squared(DVec3::NEG_Y), 0.0, 1e-10);
        let square = Square::new(2.0).with(|data| {
            data.shift(DVec3::X)
                .rotate(std::f64::consts::PI / 2.0, DVec3::Y);
        });
        assert_float_absolute_eq!(square.center.distance_squared(DVec3::X), 0.0, 1e-10);
        assert_float_absolute_eq!(square.up.distance_squared(DVec3::Y), 0.0, 1e-10);
        assert_float_absolute_eq!(square.normal.distance_squared(DVec3::X), 0.0, 1e-10);
        let square = Square::new(2.0).with(|data| {
            data.shift(DVec3::NEG_Z);
        });
        assert_float_absolute_eq!(square.center.distance_squared(DVec3::NEG_Z), 0.0, 1e-10);
        assert_float_absolute_eq!(square.up.distance_squared(DVec3::Y), 0.0, 1e-10);
        assert_float_absolute_eq!(square.normal.distance_squared(DVec3::Z), 0.0, 1e-10);
        let square = Square::new(2.0).with(|data| {
            data.shift(DVec3::Y)
                .rotate(-std::f64::consts::PI / 2.0, DVec3::X);
        });
        assert_float_absolute_eq!(square.center.distance_squared(DVec3::Y), 0.0, 1e-10);
        assert_float_absolute_eq!(square.up.distance_squared(DVec3::NEG_Z), 0.0, 1e-10);
        assert_float_absolute_eq!(square.normal.distance_squared(DVec3::Y), 0.0, 1e-10);
        let square = Square::new(2.0).with(|data| {
            data.shift(DVec3::Z);
        });
        assert_float_absolute_eq!(square.center.distance_squared(DVec3::Z), 0.0, 1e-10);
        assert_float_absolute_eq!(square.up.distance_squared(DVec3::Y), 0.0, 1e-10);
        assert_float_absolute_eq!(square.normal.distance_squared(DVec3::Z), 0.0, 1e-10);
        let square = Square::new(2.0).with(|data| {
            data.shift(DVec3::NEG_X)
                .rotate(-std::f64::consts::PI / 2.0, DVec3::Y);
        });
        assert_float_absolute_eq!(square.center.distance_squared(DVec3::NEG_X), 0.0, 1e-10);
        assert_float_absolute_eq!(square.up.distance_squared(DVec3::Y), 0.0, 1e-10);
        assert_float_absolute_eq!(square.normal.distance_squared(DVec3::NEG_X), 0.0, 1e-10);
    }
}
