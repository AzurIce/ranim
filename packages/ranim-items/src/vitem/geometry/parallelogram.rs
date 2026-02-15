use ranim_core::{
    Extract,
    color::{AlphaColor, Srgb},
    core_item::{CoreItem, vitem::DEFAULT_STROKE_WIDTH},
    glam::DVec3,
    traits::{
        Aabb, Discard, FillColor, Opacity, Rotate, Scale, Shift, StrokeColor, StrokeWidth, With,
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
    origin: DVec3,
    /// vectors representing two edges of the paralleogram
    basis: [DVec3; 2],
    /// Stroke rgba
    pub stroke_rgba: AlphaColor<Srgb>,
    /// Stroke width
    pub stroke_width: f32,
    /// Fill rgba
    pub fill_rgba: AlphaColor<Srgb>,
}

impl Parallelogram {
    /// Create a new parallelogram with the given origin and basis vectors.
    pub fn new(origin: DVec3, basis: [DVec3; 2]) -> Self {
        Self {
            origin,
            basis,
            stroke_rgba: AlphaColor::WHITE,
            stroke_width: DEFAULT_STROKE_WIDTH,
            fill_rgba: AlphaColor::TRANSPARENT,
        }
    }

    /// Get the vertices of the parallelogram.
    pub fn vertices(&self) -> [DVec3; 4] {
        let &Parallelogram {
            origin,
            basis: [u, v],
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

impl Shift for Parallelogram {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.origin += offset;
        self
    }
}

impl Rotate for Parallelogram {
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self {
        self.origin.rotate_at_point(angle, axis, point);
        self.basis.rotate_at_point(angle, axis, DVec3::ZERO);
        self
    }
}

impl Scale for Parallelogram {
    fn scale_at_point(&mut self, scale: DVec3, point: DVec3) -> &mut Self {
        self.origin.scale_at_point(scale, point);
        self.basis.iter_mut().for_each(|v| *v *= scale);
        self
    }
}

// impl AffineTransform for Parallelogram {
//     fn affine_transform_at_point(&mut self, mat: DAffine3, origin: DVec3) -> &mut Self {
//         self.origin.affine_transform_at_point(mat, origin);
//         self.basis
//             .iter_mut()
//             .for_each(|v| *v = mat.transform_vector3(*v));
//         self
//     }
// }

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
            basis,
            p0,
            size,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        } = value;
        let (u, v) = basis.uv();
        let basis = [u * size.x, v * size.y];
        Self {
            origin: p0,
            basis,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        }
    }
}

impl From<Square> for Parallelogram {
    fn from(value: Square) -> Self {
        let Square {
            basis,
            center,
            size,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        } = value;
        let (u, v) = basis.uv();
        let basis = [u * size, v * size];
        let origin = center - (u + v) * size / 2.;
        Self {
            origin,
            basis,
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
            basis,
            stroke_rgba,
            stroke_width,
            fill_rgba,
        } = value;
        let [u, v] = basis;
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
