use ranim_core::{
    Extract,
    color::{AlphaColor, Srgb},
    core_item::{CoreItem, vitem::DEFAULT_STROKE_WIDTH},
    glam::DVec3,
    traits::{
        Aabb, Discard, FillColor, Opacity, RotateTransform, ScaleTransform, ShiftTransform,
        StrokeColor, StrokeWidth, With,
    },
    utils::bezier::PathBuilder,
};

use crate::vitem::{
    VItem,
    geometry::{Polygon, Rectangle, Square},
};

/// A parallelogram.
#[derive(Debug, Clone, ranim_macros::Interpolatable)]
pub struct Parallelogram {
    /// Origin of the paralleogram
    pub origin: DVec3,
    /// vectors representing two edges of the paralleogram
    pub axes: (DVec3, DVec3),
    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
    /// Fill rgba
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Parallelogram {
    /// Create a new parallelogram with the given origin and axes vectors.
    pub fn new(origin: DVec3, axes: (DVec3, DVec3)) -> Self {
        Self {
            origin,
            axes,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }

    /// Get the vertices of the parallelogram.
    pub fn vertices(&self) -> [DVec3; 4] {
        let &Parallelogram {
            origin,
            axes: (u, v),
            ..
        } = self;
        [origin, origin + u, origin + u + v, origin + v]
    }
}

impl Aabb for Parallelogram {
    fn aabb(&self) -> [DVec3; 2] {
        self.vertices().aabb()
    }
}

impl ShiftTransform for Parallelogram {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.origin += offset;
        self
    }
}

impl RotateTransform for Parallelogram {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.origin.rotate_on_axis(axis, angle);
        self.axes.0.rotate_on_axis(axis, angle);
        self.axes.0 = self.axes.0.normalize();
        self.axes.1.rotate_on_axis(axis, angle);
        self.axes.1 = self.axes.1.normalize();
        self
    }
}

impl ScaleTransform for Parallelogram {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.origin.scale(scale).discard();
        self.axes.0 *= scale;
        self.axes.1 *= scale;
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
        self.set_stroke_opacity(opacity).set_fill_opacity(opacity);
        self
    }
}

impl From<Rectangle> for Parallelogram {
    fn from(value: Rectangle) -> Self {
        let Rectangle {
            axes,
            p0,
            size,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        } = value;
        let (u, v) = (axes.0.normalize(), axes.1.normalize());
        let axes = (u * size.x, v * size.y);
        Self {
            origin: p0,
            axes,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        }
    }
}

impl From<Square> for Parallelogram {
    fn from(value: Square) -> Self {
        let Square {
            axes,
            center,
            size,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        } = value;
        let (u, v) = (axes.0.normalize(), axes.1.normalize());
        let axes = (u * size, v * size);
        let origin = center - (u + v) * size / 2.;
        Self {
            origin,
            axes,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        }
    }
}

impl From<Parallelogram> for Polygon {
    fn from(value: Parallelogram) -> Self {
        let Parallelogram {
            stroke_rgba,
            stroke_width,
            fill_rgba,
            ..
        } = value;
        Polygon::new(value.vertices().to_vec()).with(|item| {
            item.set_stroke_color(stroke_rgba).set_fill_color(fill_rgba);
            item.stroke_width = stroke_width;
        })
    }
}

impl From<Parallelogram> for VItem {
    fn from(value: Parallelogram) -> Self {
        let Parallelogram {
            origin,
            axes: (u, v),
            stroke_rgba,
            stroke_width,
            fill_rgba,
        } = value;
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
