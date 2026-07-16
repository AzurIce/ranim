use std::f64::consts::{PI, TAU};

use ranim_core::{
    Extract,
    anchor::{BoundsAnchor, DBounds3, Locate, SemanticBounds},
    color,
    core_item::CoreItem,
    glam::{DVec2, DVec3, dvec2, dvec3},
    store::ExtractToRenderWorld,
    traits::{Discard, RotateTransform, Scale, ShiftTransform, ShiftTransformExt},
};

use color::{AlphaColor, Srgb};
use itertools::Itertools;

use crate::vitem::{DEFAULT_STROKE_WIDTH, VItem, geometry::Circle};
use ranim_core::traits::{Alignable, FillColor, Opacity, StrokeColor, StrokeWidth, With};

// MARK: ### Square ###
/// A Square
#[derive(bevy_ecs::component::Component, Clone, Debug, ranim_macros::Interpolatable)]
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
    /// Note that this accepts a `f64` scale dispite of [`Scale`]'s `DVec3`,
    /// because this keeps the square a square.
    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.scale_at(scale, BoundsAnchor::CENTER)
    }
    /// Scale the square by the given scale, with the given anchor as the center.
    ///
    /// Note that this accepts a `f64` scale dispite of [`Scale`]'s `DVec3`,
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
impl SemanticBounds for Square {
    fn semantic_bounds(&self) -> DBounds3 {
        let (u, v) = (self.axes.0.normalize(), self.axes.1.normalize());
        let half = self.size.abs() / 2.0;
        [
            self.center - half * u - half * v,
            self.center + half * u - half * v,
            self.center + half * u + half * v,
            self.center - half * u + half * v,
        ]
        .semantic_bounds()
    }
}

impl ShiftTransform for Square {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.center.shift(shift);
        self
    }
}

impl RotateTransform for Square {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.center.rotate_on_axis(axis, angle);
        self.axes.0.rotate_on_axis(axis, angle);
        self.axes.0 = self.axes.0.normalize();
        self.axes.1.rotate_on_axis(axis, angle);
        self.axes.1 = self.axes.1.normalize();
        self
    }
}

impl Scale<f64> for Square {
    fn scale(&mut self, scale: f64) -> &mut Self {
        self.size *= scale;
        self.center.scale(DVec3::splat(scale));
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

impl ExtractToRenderWorld for Square {
    type RenderItem = ranim_core::core_item::vitem::VItem;

    fn extract_to_render_world(&self, output: &mut Vec<Self::RenderItem>) {
        output.push(VItem::from(self.clone()).into());
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
}

// MARK: Traits impl
impl SemanticBounds for Rectangle {
    fn semantic_bounds(&self) -> DBounds3 {
        let (u, v) = (self.axes.0.normalize(), self.axes.1.normalize());
        let p0 = self.p0;
        let p1 = p0 + self.size.x * u;
        let p2 = p1 + self.size.y * v;
        let p3 = p0 + self.size.y * v;
        [p0, p1, p2, p3].semantic_bounds()
    }
}

impl ShiftTransform for Rectangle {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.p0.shift(shift);
        self
    }
}

impl RotateTransform for Rectangle {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.p0.rotate_on_axis(axis, angle);
        self.axes.0.rotate_on_axis(axis, angle);
        self.axes.0 = self.axes.0.normalize();
        self.axes.1.rotate_on_axis(axis, angle);
        self.axes.1 = self.axes.1.normalize();
        self
    }
}

impl Scale for Rectangle {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.p0.scale(scale);
        let (u, v) = (self.axes.0.normalize(), self.axes.1.normalize());
        let scale_u = scale.dot(u);
        let scale_v = scale.dot(v);
        self.size *= dvec2(scale_u, scale_v);
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
impl SemanticBounds for Polygon {
    fn semantic_bounds(&self) -> DBounds3 {
        self.points.semantic_bounds()
    }
}

impl ShiftTransform for Polygon {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.points.shift(shift);
        self
    }
}

impl RotateTransform for Polygon {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.points.rotate_on_axis(axis, angle);
        self.axes.0.rotate_on_axis(axis, angle);
        self.axes.0 = self.axes.0.normalize();
        self.axes.1.rotate_on_axis(axis, angle);
        self.axes.1 = self.axes.1.normalize();
        self
    }
}

impl Scale for Polygon {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.points.scale(scale);
        self
    }
}

// impl AffineTransform for Polygon {
//     fn affine_transform_at_point(&mut self, mat: DAffine3, origin: DVec3) -> &mut Self {
//         self.points.affine_transform_at_point(mat, origin);
//         // TODO: how to transform axes?
//         self
//     }
// }

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
            axes,
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
        VItem::from_vpoints(vpoints)
            .with_normal(axes.0.cross(axes.1).normalize())
            .with(|vitem| {
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

impl SemanticBounds for RegularPolygon {
    fn semantic_bounds(&self) -> DBounds3 {
        self.points().semantic_bounds()
    }
}

impl ShiftTransform for RegularPolygon {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.center.shift(offset);
        self
    }
}

impl RotateTransform for RegularPolygon {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.axes.0.rotate_on_axis(axis, angle);
        self.axes.0 = self.axes.0.normalize();
        self.axes.1.rotate_on_axis(axis, angle);
        self.axes.1 = self.axes.1.normalize();
        self.center.rotate_on_axis(axis, angle);
        self
    }
}

impl Scale<f64> for RegularPolygon {
    fn scale(&mut self, scale: f64) -> &mut Self {
        self.radius *= scale;
        self.center.scale(DVec3::splat(scale));
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
mod render_world_tests {
    use ranim_core::store::CoreItemStore;

    use super::*;

    #[test]
    fn square_can_live_in_main_world_and_extract_to_render_world() {
        let mut store = CoreItemStore::new();
        let entity = store.insert_item(Square::new(2.0));

        assert!(store.world().get::<Square>(entity).is_some());
        assert_eq!(store.render_world().vitems().count(), 1);
    }
}
