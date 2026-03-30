use std::f64::consts::{PI, TAU};

use ranim_core::{
    anchor::{Aabb, AabbPoint, Anchor},
    components::vpoint::VPointVec,
    core_item::vitem::Basis2d,
    glam::{DVec2, DVec3, dvec2, dvec3},
    traits::{Discard, RotateTransform, ScaleTransform, ShiftTransform, ShiftTransformExt},
};

use itertools::Itertools;

use crate::vitem::{VItem, VPath};
use crate::vitem::geometry::Circle;

// MARK: ### Square ###
/// A Square
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Square {
    /// Basis
    pub basis: Basis2d,
    /// Center
    pub center: DVec3,
    /// Size
    pub size: f64,
}

impl VItem<Square> {
    /// Constructor
    pub fn new(size: f64) -> Self {
        Self::new_with(Square {
            basis: Basis2d::default(),
            center: dvec3(0.0, 0.0, 0.0),
            size,
        })
    }
    /// Scale the square by the given scale, with the given anchor as the center.
    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.scale_at(scale, AabbPoint::CENTER)
    }
    /// Scale the square by the given scale, with the given anchor as the center.
    pub fn scale_at<T>(&mut self, scale: f64, anchor: T) -> &mut Self
    where
        T: Anchor<Self>,
    {
        let anchor = anchor.locate_on(self);
        self.with_inner_mut(|square| {
            square.size *= scale;
            square.center
                .shift(-anchor)
                .scale(DVec3::splat(scale))
                .shift(anchor);
        });
        self
    }
}

// MARK: Traits impl
impl Aabb for Square {
    fn aabb(&self) -> [DVec3; 2] {
        let (u, v) = self.basis.uv();
        [
            self.center + self.size / 2.0 * (u + v),
            self.center - self.size / 2.0 * (u + v),
        ]
        .aabb()
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
        self.basis.rotate_on_axis(axis, angle);
        self
    }
}

// MARK: Conversions
impl From<Square> for Rectangle {
    fn from(value: Square) -> Self {
        let Square {
            basis,
            center,
            size: width,
        } = value;
        let (u, v) = basis.uv();
        let p0 = center - width / 2.0 * u - width / 2.0 * v;
        Rectangle {
            basis,
            p0,
            size: dvec2(width, width),
        }
    }
}

impl From<Square> for RegularPolygon {
    fn from(value: Square) -> Self {
        let mut rp = RegularPolygon::new(4, value.size / 2.0 * 2.0f64.sqrt());
        rp.basis = value.basis;
        rp.center = value.center;
        rp
    }
}

impl From<Square> for Polygon {
    fn from(value: Square) -> Self {
        Rectangle::from(value).into()
    }
}

impl VPath for Square {
    fn normal(&self) -> DVec3 {
        self.basis.normal()
    }
    fn build_vpoint_vec(&self) -> VPointVec {
        Rectangle::from(self.clone()).build_vpoint_vec()
    }
}

// MARK: ### Rectangle ###
/// Rectangle
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Rectangle {
    /// Basis info
    pub basis: Basis2d,
    /// Bottom left corner (minimum)
    pub p0: DVec3,
    /// Width and height
    pub size: DVec2,
}

impl VItem<Rectangle> {
    /// Constructor
    pub fn new(width: f64, height: f64) -> Self {
        let half_width = width / 2.0;
        let half_height = height / 2.0;
        let p0 = dvec3(-half_width, -half_height, 0.0);
        let size = dvec2(width, height);
        Self::new_with(Rectangle::from_min_size(p0, size))
    }
}

impl Rectangle {
    /// Construct a rectangle from the bottom-left point (minimum) and size.
    pub fn from_min_size(p0: DVec3, size: DVec2) -> Self {
        Self {
            basis: Basis2d::default(),
            p0,
            size,
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
impl Aabb for Rectangle {
    fn aabb(&self) -> [DVec3; 2] {
        let (u, v) = self.basis.uv();
        let p1 = self.p0;
        let p2 = self.p0 + self.size.x * u + self.size.y * v;
        [p1, p2].aabb()
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
        self.basis.rotate_on_axis(axis, angle);
        self
    }
}

impl ScaleTransform for Rectangle {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.p0.scale(scale);
        let (u, v) = self.basis.uv();
        let scale_u = scale.dot(u);
        let scale_v = scale.dot(v);
        self.size *= dvec2(scale_u, scale_v);
        self
    }
}

// MARK: Conversions
impl From<Rectangle> for Polygon {
    fn from(value: Rectangle) -> Self {
        let p0 = value.p0;
        let (u, v) = value.basis.uv();
        let DVec2 { x: w, y: h } = value.size;
        let points = vec![p0, p0 + u * w, p0 + u * w + v * h, p0 + v * h];
        Polygon {
            basis: value.basis,
            points,
        }
    }
}

impl VPath for Rectangle {
    fn normal(&self) -> DVec3 {
        self.basis.normal()
    }
    fn build_vpoint_vec(&self) -> VPointVec {
        Polygon::from(self.clone()).build_vpoint_vec()
    }
}

// MARK: ### Polygon ###
/// A Polygon
#[derive(Clone, Debug)]
pub struct Polygon {
    /// Basis info
    pub basis: Basis2d,
    /// Corner points
    pub points: Vec<DVec3>,
}

impl VItem<Polygon> {
    /// Constructor
    pub fn new(points: Vec<DVec3>) -> Self {
        Self::new_with(Polygon {
            basis: Basis2d::default(),
            points,
        })
    }
}

impl Polygon {
    /// Constructor
    pub fn new(points: Vec<DVec3>) -> Self {
        Self {
            basis: Basis2d::default(),
            points,
        }
    }
}

// MARK: Traits impl
impl Aabb for Polygon {
    fn aabb(&self) -> [DVec3; 2] {
        self.points.aabb()
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
        self.basis.rotate_on_axis(axis, angle);
        self
    }
}

impl ScaleTransform for Polygon {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.points.scale(scale);
        self
    }
}

impl ranim_core::traits::Interpolatable for Polygon {
    fn lerp(&self, other: &Self, t: f64) -> Self {
        use ranim_core::traits::Interpolatable;
        Self {
            basis: Interpolatable::lerp(&self.basis, &other.basis, t),
            points: Interpolatable::lerp(&self.points, &other.points, t),
        }
    }
    fn is_aligned(&self, other: &Self) -> bool {
        self.points.len() == other.points.len()
    }
    fn align_with(&mut self, other: &mut Self) {
        if self.points.len() > other.points.len() {
            return other.align_with(self);
        }
        self.points
            .resize(other.points.len(), self.points.last().cloned().unwrap());
    }
}

impl VPath for Polygon {
    fn normal(&self) -> DVec3 {
        self.basis.normal()
    }
    fn build_vpoint_vec(&self) -> VPointVec {
        assert!(self.points.len() > 2);

        let mut points = self.points.clone();
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
        VPointVec(vpoints)
    }
}

// MARK: ### RegularPolygon ###
#[derive(Debug, Clone, ranim_macros::Interpolatable)]
/// A regular polygon.
pub struct RegularPolygon {
    /// Local coordinate system
    pub basis: Basis2d,
    /// Center of the polygon
    pub center: DVec3,
    /// Number of sides
    pub sides: usize,
    /// Radius of the polygon (i.e. distance from center to a vertex)
    pub radius: f64,
}

impl VItem<RegularPolygon> {
    /// Creates a new regular polygon.
    pub fn new(sides: usize, radius: f64) -> Self {
        assert!(sides >= 3);
        Self::new_with(RegularPolygon {
            basis: Basis2d::default(),
            center: DVec3::ZERO,
            sides,
            radius,
        })
    }
}

impl RegularPolygon {
    /// Creates a new regular polygon.
    pub fn new(sides: usize, radius: f64) -> Self {
        assert!(sides >= 3);
        Self {
            basis: Basis2d::default(),
            center: DVec3::ZERO,
            sides,
            radius,
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
        let u = self.basis.u();
        let normal = self.basis.normal();
        (0..sides)
            .map(|i| TAU * (i as f64 / sides as f64))
            .map(|angle| u.rotate_axis(normal, angle) * radius + center)
            .collect()
    }
    /// Returns the outer circle of the polygon.
    pub fn outer_circle(&self) -> Circle {
        use ranim_core::traits::With;
        Circle::new(self.radius).with(|x| x.move_to(self.center).discard())
    }
    /// Returns the inner circle of the polygon.
    pub fn inner_circle(&self) -> Circle {
        use ranim_core::traits::With;
        Circle::new(self.radius * (PI / self.sides as f64).cos())
            .with(|x| x.move_to(self.center).discard())
    }
}

impl Aabb for RegularPolygon {
    fn aabb(&self) -> [DVec3; 2] {
        self.points().aabb()
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
        self.basis.rotate_on_axis(axis, angle);
        self.center.rotate_on_axis(axis, angle);
        self
    }
}

impl From<RegularPolygon> for Polygon {
    fn from(value: RegularPolygon) -> Self {
        let mut p = Polygon::new(value.points());
        p.basis = value.basis;
        p
    }
}

impl VPath for RegularPolygon {
    fn normal(&self) -> DVec3 {
        self.basis.normal()
    }
    fn build_vpoint_vec(&self) -> VPointVec {
        Polygon::from(self.clone()).build_vpoint_vec()
    }
}
