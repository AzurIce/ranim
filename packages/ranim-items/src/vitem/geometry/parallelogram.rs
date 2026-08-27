use ranim_core::{
    Extract,
    anchor::Aabb,
    color::{AlphaColor, Srgb},
    core_item::{CoreItem, vitem::DEFAULT_STROKE_WIDTH},
    glam::{DAffine3, DVec3},
    traits::{ApplyTransform, Discard, FillColor, Opacity, StrokeColor, StrokeWidth, With},
    utils::bezier::PathBuilder,
};

use crate::vitem::{
    VItem,
    geometry::{Polygon, Rectangle, Square},
};

/// A general affine parallelogram in local coordinates.
#[derive(Debug, Clone, ranim_macros::Interpolatable)]
#[allow(missing_docs)]
pub struct Parallelogram {
    /// Local origin.
    pub origin: DVec3,
    /// Local edge vectors.
    pub axes: (DVec3, DVec3),
    pub stroke_rgba: AlphaColor<Srgb>,
    pub stroke_width: f32,
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Parallelogram {
    /// Creates a parallelogram with a zero local origin.
    pub fn new(axes: (DVec3, DVec3)) -> Self {
        Self::from_origin_and_axes(DVec3::ZERO, axes)
    }
    /// Creates a parallelogram with an explicit local origin and edge vectors.
    pub fn from_origin_and_axes(origin: DVec3, axes: (DVec3, DVec3)) -> Self {
        Self {
            origin,
            axes,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }
    /// Returns the four vertices in winding order.
    pub fn vertices(&self) -> [DVec3; 4] {
        let (u, v) = self.axes;
        [
            self.origin,
            self.origin + u,
            self.origin + u + v,
            self.origin + v,
        ]
    }
}
impl Aabb for Parallelogram {
    fn aabb(&self) -> [DVec3; 2] {
        self.vertices().aabb()
    }
}
impl<G: Into<DAffine3>> ApplyTransform<G> for Parallelogram {
    fn apply(&mut self, transform: G) -> &mut Self {
        let t = transform.into();
        self.origin = t.transform_point3(self.origin);
        self.axes.0 = t.transform_vector3(self.axes.0);
        self.axes.1 = t.transform_vector3(self.axes.1);
        self
    }
}
impl StrokeColor for Parallelogram {
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
impl FillColor for Parallelogram {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.fill_rgba
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rgba = self.fill_rgba.with_alpha(opacity);
        self
    }
    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.fill_rgba = color;
        self
    }
}
impl Opacity for Parallelogram {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.set_stroke_opacity(opacity).set_fill_opacity(opacity)
    }
}

impl From<Rectangle> for Parallelogram {
    fn from(value: Rectangle) -> Self {
        let (w, h) = (value.size.x, value.size.y);
        Self {
            origin: DVec3::new(-w / 2.0, -h / 2.0, 0.0),
            axes: (DVec3::X * w, DVec3::Y * h),
            stroke_rgba: value.stroke_rgba,
            stroke_width: value.stroke_width,
            fill_rgba: value.fill_rgba,
        }
    }
}
impl From<Square> for Parallelogram {
    fn from(value: Square) -> Self {
        Parallelogram::from(Rectangle::from(value))
    }
}
impl From<Parallelogram> for Polygon {
    fn from(value: Parallelogram) -> Self {
        let p = value.vertices();
        Polygon::new(p.to_vec()).with(|x| {
            x.set_stroke_color(value.stroke_rgba)
                .set_fill_color(value.fill_rgba);
            x.stroke_width = value.stroke_width;
        })
    }
}
impl From<Parallelogram> for VItem {
    fn from(value: Parallelogram) -> Self {
        let (origin, (u, v), stroke_rgba, stroke_width, fill_rgba) = (
            value.origin,
            value.axes,
            value.stroke_rgba,
            value.stroke_width,
            value.fill_rgba,
        );
        VItem::from_vpoints(
            PathBuilder::new()
                .move_to(origin)
                .line_to(origin + u)
                .line_to(origin + u + v)
                .line_to(origin + v)
                .close_path()
                .vpoints()
                .into(),
        )
        .with(|item| {
            item.set_fill_color(fill_rgba)
                .set_stroke_color(stroke_rgba)
                .set_stroke_width(stroke_width)
                .discard()
        })
    }
}
impl Extract for Parallelogram {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        VItem::from(self.clone()).extract_into(buf)
    }
}
