use std::f64::consts::{PI, TAU};

use ranim_core::{
    Extract,
    anchor::{Aabb, AabbPoint, Locate},
    color,
    core_item::CoreItem,
    glam::{DVec2, DVec3, dvec2, dvec3},
    traits::{ApplyTransform, Discard, ScaleTransform, ShiftTransform, ShiftTransformExt},
};

use color::{AlphaColor, Srgb};
use itertools::Itertools;

use crate::vitem::{DEFAULT_STROKE_WIDTH, VItem, geometry::Circle};
use ranim_core::traits::{Alignable, FillColor, Opacity, StrokeColor, StrokeWidth, With};

// MARK: ### Square ###
/// A Square
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Square {
    /// Axes
    pub axes: (DVec3, DVec3),
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
            axes: (DVec3::X, DVec3::Y),
            center: dvec3(0.0, 0.0, 0.0),
            size,

            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
    /// Scale the square by the given scale, with the given anchor as the center.
    ///
    /// Note that this accepts a `f64` scale dispite of [`ScaleTransform`]'s `DVec3`,
    /// because this keeps the square a square.
    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.scale_at(scale, AabbPoint::CENTER)
    }
    /// Scale the square by the given scale, with the given anchor as the center.
    ///
    /// Note that this accepts a `f64` scale dispite of [`ScaleTransform`]'s `DVec3`,
    /// because this keeps the square a square.
    pub fn scale_at<T>(&mut self, scale: f64, anchor: T) -> &mut Self
    where
        T: Locate<Self>,
    {
        let anchor = anchor.locate(self);
        self.size *= scale;
        self.center
            .shift(-anchor)
            .scale(DVec3::splat(scale))
            .shift(anchor);
        self
    }
}

// MARK: Traits impl
impl Aabb for Square {
    fn aabb(&self) -> [DVec3; 2] {
        let (u, v) = (self.axes.0.normalize(), self.axes.1.normalize());
        [
            self.center + self.size / 2.0 * (u + v),
            self.center - self.size / 2.0 * (u + v),
        ]
        .aabb()
    }
}

impl<G: Into<ranim_core::prelude::Similarity>> ApplyTransform<G> for Square {
    fn apply(&mut self, transform: G) -> &mut Self {
        let transform = transform.into();
        self.center = transform.transform_point(self.center);
        self.axes.0 = transform.transform_direction(self.axes.0);
        self.axes.1 = transform.transform_direction(self.axes.1);
        self.size *= transform.scale;
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
            axes,
            center,
            size: width,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        } = value;
        let (u, v) = (axes.0.normalize(), axes.1.normalize());
        let p0 = center - width / 2.0 * u - width / 2.0 * v;
        Rectangle {
            axes,
            p0,
            size: dvec2(width, width),
            stroke_rgba,
            stroke_width,
            fill_rgba,
        }
    }
}

impl From<Square> for RegularPolygon {
    fn from(value: Square) -> Self {
        RegularPolygon::new(4, value.size / 2.0 * 2.0f64.sqrt()).with(|x| {
            x.axes = value.axes;
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

// MARK: ### Rectangle ###
/// Rectangle
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Rectangle {
    /// Axes info
    pub axes: (DVec3, DVec3),
    /// Bottom left corner (minimum)
    pub p0: DVec3,
    /// Width and height
    pub size: DVec2,

    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
    /// Fill rgba
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Rectangle {
    /// Constructor
    pub fn new(width: f64, height: f64) -> Self {
        let half_width = width / 2.0;
        let half_height = height / 2.0;
        let p0 = dvec3(-half_width, -half_height, 0.0);
        let size = dvec2(width, height);
        Self::from_min_size(p0, size)
    }
    /// Construct a rectangle from the bottom-left point (minimum) and size.
    pub fn from_min_size(p0: DVec3, size: DVec2) -> Self {
        Self {
            axes: (DVec3::X, DVec3::Y),
            p0,
            size,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
    /// Width
    pub fn width(&self) -> f64 {
        self.size.x.abs()
    }
    /// Height
    pub fn height(&self) -> f64 {
        self.size.y.abs()
    }

    /// Scale the dimensions along the rectangle's stored shape axes.
    ///
    /// This edits `size` without changing the stored orthogonal `axes`. To
    /// compose a transform outside or inside a wrapper instead, use
    /// [`ranim_core::core_item::transformed::Transformed::compose_outer`] or
    /// [`ranim_core::core_item::transformed::Transformed::compose_inner`].
    pub fn scale_axes(&mut self, scale: DVec2) -> &mut Self {
        self.size *= scale;
        self
    }
}

// MARK: Traits impl
impl Aabb for Rectangle {
    fn aabb(&self) -> [DVec3; 2] {
        let (u, v) = (self.axes.0.normalize(), self.axes.1.normalize());
        let p1 = self.p0;
        let p2 = self.p0 + self.size.x * u + self.size.y * v;
        [p1, p2].aabb()
    }
}

impl<G: Into<ranim_core::prelude::Similarity>> ApplyTransform<G> for Rectangle {
    fn apply(&mut self, transform: G) -> &mut Self {
        let transform = transform.into();
        self.p0 = transform.transform_point(self.p0);
        self.axes.0 = transform.transform_direction(self.axes.0);
        self.axes.1 = transform.transform_direction(self.axes.1);
        self.size *= transform.scale;
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
        let p0 = value.p0;
        let (u, v) = (value.axes.0.normalize(), value.axes.1.normalize());
        let DVec2 { x: w, y: h } = value.size;
        let points = vec![p0, p0 + u * w, p0 + u * w + v * h, p0 + v * h];
        Polygon {
            axes: value.axes,
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
    /// Axes info
    pub axes: (DVec3, DVec3),
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
            axes: (DVec3::X, DVec3::Y),
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

impl<G: Into<ranim_core::glam::DAffine3>> ApplyTransform<G> for Polygon {
    fn apply(&mut self, transform: G) -> &mut Self {
        let transform = transform.into();
        self.points.apply(transform);
        self.axes.0 = transform.transform_vector3(self.axes.0);
        self.axes.1 = transform.transform_vector3(self.axes.1);
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
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        VItem::from(self.clone()).extract_into(buf);
    }
}

#[derive(Debug, Clone, ranim_macros::Interpolatable)]
/// A regular polygon.
pub struct RegularPolygon {
    /// Local coordinate system
    pub axes: (DVec3, DVec3),
    /// Center of the polygon
    pub center: DVec3,
    /// Number of sides
    pub sides: usize,
    /// Radius of the polygon (i.e. distance from center to a vertex)
    pub radius: f64,
    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
    /// Fill rgba
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Alignable for RegularPolygon {
    fn is_aligned(&self, _other: &Self) -> bool {
        true
    }
    fn align_with(&mut self, _other: &mut Self) {}
}

impl RegularPolygon {
    /// Creates a new regular polygon.
    pub fn new(sides: usize, radius: f64) -> Self {
        assert!(sides >= 3);
        Self {
            axes: (DVec3::X, DVec3::Y),
            center: DVec3::ZERO,
            sides,
            radius,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
    /// Returns the vertices of the polygon.
    pub fn points(&self) -> Vec<DVec3> {
        let &Self {
            sides,
            radius,
            center,
            ..
        } = self;
        let u = self.axes.0.normalize();
        let normal = self.axes.0.cross(self.axes.1).normalize();
        (0..sides)
            .map(|i| TAU * (i as f64 / sides as f64))
            .map(|angle| u.rotate_axis(normal, angle) * radius + center)
            .collect()
    }
    /// Returns the outer circle of the polygon.
    pub fn outer_circle(&self) -> Circle {
        Circle::new(self.radius).with(|x| x.move_to(self.center).discard())
    }
    /// Returns the inner circle of the polygon.
    pub fn inner_circle(&self) -> Circle {
        Circle::new(self.radius * (PI / self.sides as f64).cos())
            .with(|x| x.move_to(self.center).discard())
    }
}

impl Aabb for RegularPolygon {
    fn aabb(&self) -> [DVec3; 2] {
        self.points().aabb()
    }
}

impl<G: Into<ranim_core::prelude::Similarity>> ApplyTransform<G> for RegularPolygon {
    fn apply(&mut self, transform: G) -> &mut Self {
        let transform = transform.into();
        self.center = transform.transform_point(self.center);
        self.axes.0 = transform.transform_direction(self.axes.0);
        self.axes.1 = transform.transform_direction(self.axes.1);
        self.radius *= transform.scale;
        self
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
            x.axes = value.axes;
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

#[cfg(test)]
mod tests {
    use super::*;
    use ranim_core::traits::RotateTransform;

    #[test]
    fn rotated_rectangle_scale_axes_preserves_stored_axes() {
        let mut rectangle = Rectangle::new(2.0, 3.0);
        rectangle.rotate_on_axis(DVec3::Z, core::f64::consts::FRAC_PI_4);
        let axes = rectangle.axes;
        let p0 = rectangle.p0;

        rectangle.scale_axes(dvec2(2.0, 0.5));

        assert_eq!(rectangle.axes, axes);
        assert_eq!(rectangle.p0, p0);
        assert_eq!(rectangle.size, dvec2(4.0, 1.5));
        assert!(rectangle.axes.0.dot(rectangle.axes.1).abs() < 1e-12);
    }
}
