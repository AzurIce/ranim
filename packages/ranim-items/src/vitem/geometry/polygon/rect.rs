use ranim_core::{
    Extract,
    anchor::Aabb,
    color,
    core_item::CoreItem,
    glam::{self, Vec3Swizzles},
    proj::CoordinateSystem,
    traits::{
        LocalCoordinate, Origin, Rotate, RotateLocal, ScaleByProj, ScaleLocal, ScaleUniform,
        ScaleUniformByOrigin, ScaleUniformLocal, Shift, With,
    },
};

use color::{AlphaColor, Srgb};
use glam::{DVec2, DVec3, dvec2, dvec3};

use super::Polygon;
use crate::vitem::{DEFAULT_STROKE_WIDTH, VItem};
use ranim_core::traits::{Alignable, FillColor, Opacity, StrokeColor};

// MARK: ### Rectangle ###
/// Rectangle
#[derive(Clone, Debug, ranim_macros::Interpolatable)]
pub struct Rectangle {
    /// Local coordinate system
    pub coord: CoordinateSystem,
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
            coord: CoordinateSystem::default().with(|coord| {
                coord.move_to(p0);
            }),
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

impl Origin for Rectangle {
    fn origin(&self) -> DVec3 {
        self.coord.origin
    }
    fn move_to(&mut self, origin: DVec3) -> &mut Self {
        self.coord.move_to(origin);
        self
    }
}

// MARK: Traits impl
impl LocalCoordinate for Rectangle {
    fn coord(&self) -> CoordinateSystem {
        self.coord
    }
}

impl Aabb for Rectangle {
    fn aabb(&self) -> [DVec3; 2] {
        let (u, v) = self.coord.basis();
        let p1 = self.coord.origin;
        let p2 = p1 + self.size.x * u + self.size.y * v;
        [p1, p2].aabb()
    }
}

impl Shift for Rectangle {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.coord.shift(shift);
        self
    }
}

impl Rotate for Rectangle {
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self {
        self.coord.rotate_at_point(angle, axis, point);
        self
    }
}

impl RotateLocal for Rectangle {
    fn rotate_local(&mut self, angle: f64) -> &mut Self {
        self.coord.rotate_local(angle);
        self
    }
}

impl ScaleUniform for Rectangle {
    fn scale_uniform_at_point(&mut self, scale: f64, point: DVec3) -> &mut Self {
        self.coord.origin.scale_uniform_at_point(scale, point);
        self.size *= scale;
        self
    }
}

impl ScaleUniformByOrigin for Rectangle {
    fn scale_uniform(&mut self, scale: f64) -> &mut Self {
        self.size *= scale;
        self
    }
}

impl ScaleUniformLocal for Rectangle {}

impl ScaleLocal for Rectangle {
    fn scale_local_at_coord(&mut self, scale: DVec3, coord: DVec3) -> &mut Self {
        self.scale_local_at_point(scale, self.coord.c2p(coord));
        self
    }
    fn scale_local_at_point(&mut self, scale: DVec3, point: DVec3) -> &mut Self {
        self.coord
            .origin
            .scale_by_proj_at_point(scale, point, self.coord.proj);
        self
    }
    fn scale_local(&mut self, scale: DVec3) -> &mut Self {
        self.size *= scale.xy();
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
        let p0 = value.origin();
        let (u, v) = value.coord().basis();
        let DVec2 { x: w, y: h } = value.size;
        let points = vec![p0, p0 + u * w, p0 + u * w + v * h, p0 + v * h];
        Polygon {
            proj: value.proj(),
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
