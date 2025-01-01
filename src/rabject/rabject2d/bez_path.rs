use std::{
    cmp::Ordering,
    ops::{Deref, DerefMut},
};

use bevy_color::LinearRgba;
use glam::FloatExt;
use itertools::Itertools;
use vello::{
    kurbo::{self, Affine, CubicBez, Line, PathSeg, QuadBez},
    peniko::{self, color::AlphaColor, Brush},
};

use crate::{
    prelude::{Alignable, Interpolatable, Opacity},
    scene::{canvas::camera::CanvasCamera, Entity},
    utils::bezier::divide_segment_to_n_part,
};

use super::vmobject::VMobject;

#[derive(Clone, Debug)]
pub struct BezPath {
    pub inner: kurbo::BezPath,
    pub stroke: StrokeOptions,
    pub fill: FillOptions,
}

impl Deref for BezPath {
    type Target = kurbo::BezPath;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for BezPath {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Into<VMobject> for BezPath {
    fn into(self) -> VMobject {
        VMobject::new(vec![self])
    }
}

impl BezPath {
    pub fn get_matched_segments(&mut self, len: usize) -> Vec<PathSeg> {
        let mut lens = self
            .inner
            .segments()
            .map(|seg| match seg {
                kurbo::PathSeg::Line(Line { p0, p1 }) => p0.distance(p1),
                kurbo::PathSeg::Quad(QuadBez { p0, p2, .. }) => p0.distance(p2),
                kurbo::PathSeg::Cubic(CubicBez { p0, p3, .. }) => p0.distance(p3),
            })
            .collect::<Vec<_>>();
        // println!("get_matched_segments {} from {} {}", len, self.inner.segments().try_len().unwrap_or(0), lens.len());

        let n = len - lens.len();
        let mut ipc = vec![0; lens.len()];
        for _ in 0..n {
            let i = lens
                .iter()
                .position_max_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
                .unwrap();
            ipc[i] += 1;
            lens[i] *= ipc[i] as f64 / (ipc[i] + 1) as f64;
        }

        let mut new_segments = Vec::with_capacity(len);
        self.inner.segments().zip(ipc).for_each(|(seg, ipc)| {
            if ipc > 0 {
                let divided = divide_segment_to_n_part(seg, ipc + 1);
                new_segments.extend(divided)
            } else {
                new_segments.push(seg)
            }
        });

        new_segments
    }
}

/// An [`PathSeg`] is aligned if it has the same type as the other [`PathSeg`].
impl Alignable for PathSeg {
    fn is_aligned(&self, other: &Self) -> bool {
        match (self, other) {
            (PathSeg::Line(_), PathSeg::Line(_))
            | (PathSeg::Quad(_), PathSeg::Quad(_))
            | (PathSeg::Cubic(_), PathSeg::Cubic(_)) => true,
            _ => false,
        }
    }
    fn align_with(&mut self, other: &mut Self) {
        if !self.is_aligned(other) {
            if let PathSeg::Line(line) = self {
                *self = PathSeg::Quad(QuadBez {
                    p0: line.p0,
                    p1: line.p0.midpoint(line.p1),
                    p2: line.p1,
                });
            }
            if let PathSeg::Line(line) = other {
                *other = PathSeg::Quad(QuadBez {
                    p0: line.p0,
                    p1: line.p0.midpoint(line.p1),
                    p2: line.p1,
                });
            }
        }

        if !self.is_aligned(other) {
            if let PathSeg::Quad(quad) = self {
                *self = PathSeg::Cubic(quad.raise())
            }
            if let PathSeg::Quad(quad) = other {
                *other = PathSeg::Cubic(quad.raise())
            }
        }
    }
}

impl Interpolatable for Line {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Line {
            p0: self.p0.lerp(target.p0, t as f64),
            p1: self.p1.lerp(target.p1, t as f64),
        }
    }
}

impl Interpolatable for QuadBez {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        QuadBez {
            p0: self.p0.lerp(target.p0, t as f64),
            p1: self.p1.lerp(target.p1, t as f64),
            p2: self.p2.lerp(target.p2, t as f64),
        }
    }
}

impl Interpolatable for CubicBez {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        CubicBez {
            p0: self.p0.lerp(target.p0, t as f64),
            p1: self.p1.lerp(target.p1, t as f64),
            p2: self.p2.lerp(target.p2, t as f64),
            p3: self.p3.lerp(target.p3, t as f64),
        }
    }
}

impl Interpolatable for PathSeg {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        // After aligned, the segment should be in the same type
        match (self, target) {
            (PathSeg::Line(a), PathSeg::Line(b)) => PathSeg::Line(a.lerp(b, t)),
            (PathSeg::Quad(a), PathSeg::Quad(b)) => PathSeg::Quad(a.lerp(b, t)),
            (PathSeg::Cubic(a), PathSeg::Cubic(b)) => PathSeg::Cubic(a.lerp(b, t)),
            _ => unreachable!(),
        }
    }
}

impl Alignable for BezPath {
    fn is_aligned(&self, other: &Self) -> bool {
        self.inner.segments().count() == other.inner.segments().count()
            && self
                .inner
                .segments()
                .zip(other.inner.segments())
                .all(|(a, b)| a.is_aligned(&b))
    }

    fn align_with(&mut self, other: &mut Self) {
        let self_len = self.inner.segments().count();
        let other_len = other.inner.segments().count();
        // println!(">>>> aligning BezPath {} {}", self_len, other_len);
        let len = self_len.max(other_len);

        let mut self_segs = self.get_matched_segments(len);
        let mut other_segs = other.get_matched_segments(len);
        // println!("<<<< aligned BezPath {} {}", self_segs.len(), other_segs.len());

        self_segs
            .iter_mut()
            .zip(other_segs.iter_mut())
            .for_each(|(a, b)| {
                a.align_with(b);
            });
        self.inner = kurbo::BezPath::from_path_segments(self_segs.into_iter());
        other.inner = kurbo::BezPath::from_path_segments(other_segs.into_iter());
    }
}

impl Interpolatable for kurbo::BezPath {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        // After aligned, the segments should have same length and each segment should be in the same type
        let segments = self
            .segments()
            .zip(target.segments())
            .map(|(a, b)| a.lerp(&b, t));
        kurbo::BezPath::from_path_segments(segments)
    }
}

impl Interpolatable for BezPath {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        BezPath {
            inner: self.inner.lerp(&other.inner, t),
            stroke: self.stroke.lerp(&other.stroke, t),
            fill: self.fill.lerp(&other.fill, t),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StrokeOptions {
    pub opacity: f32, // Because we can't get opacity from peniko's Brush
    pub style: kurbo::Stroke,
    pub transform: Option<Affine>,
    pub brush: Brush,
}

impl StrokeOptions {
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self.brush = self.brush.clone().with_alpha(opacity);
        self
    }
    pub fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.opacity = opacity;
        self.brush = self.brush.clone().with_alpha(opacity);
        self
    }
}

impl Interpolatable for kurbo::Stroke {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        kurbo::Stroke {
            width: self.width.lerp(target.width, t as f64),
            miter_limit: self.miter_limit.lerp(target.miter_limit, t as f64),
            join: if t == 0.0 { self.join } else { target.join },
            start_cap: if t == 0.0 {
                self.start_cap
            } else {
                target.start_cap
            },
            end_cap: if t == 0.0 {
                self.end_cap
            } else {
                target.end_cap
            },
            dash_pattern: if t == 0.0 {
                self.dash_pattern.clone()
            } else {
                target.dash_pattern.clone()
            },
            dash_offset: self.dash_offset.lerp(target.dash_offset, t as f64),
        }
    }
}

impl Interpolatable for peniko::Brush {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        if let (peniko::Brush::Solid(a), peniko::Brush::Solid(b)) = (self, target) {
            return peniko::Brush::Solid(a.lerp(*b, t, peniko::color::HueDirection::Shorter));
        }
        if t == 0.0 {
            self.clone()
        } else {
            target.clone()
        }
    }
}

impl Interpolatable for StrokeOptions {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        StrokeOptions {
            opacity: self.opacity.lerp(target.opacity, t),
            style: self.style.lerp(&target.style, t),
            transform: if t == 0.0 {
                self.transform
            } else if t == 1.0 {
                target.transform
            } else {
                self.transform
            },
            brush: self.brush.lerp(&target.brush, t),
        }
    }
}

impl Default for StrokeOptions {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            style: kurbo::Stroke {
                start_cap: kurbo::Cap::Square,
                end_cap: kurbo::Cap::Square,
                ..Default::default()
            },
            transform: None,
            brush: Brush::Solid(peniko::color::palette::css::RED),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FillOptions {
    pub opacity: f32, // Because we can't get opacity from peniko's Brush
    pub style: peniko::Fill,
    pub transform: Option<Affine>,
    pub brush: Brush,
}

impl FillOptions {
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self.brush = self.brush.clone().with_alpha(opacity);
        self
    }
    pub fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.opacity = opacity;
        self.brush = self.brush.clone().with_alpha(opacity);
        self
    }
}

impl Interpolatable for FillOptions {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        FillOptions {
            opacity: self.opacity.lerp(target.opacity, t),
            style: if t == 0.0 { self.style } else { target.style },
            transform: if t == 0.0 {
                self.transform
            } else {
                target.transform
            },
            brush: self.brush.lerp(&target.brush, t),
        }
    }
}

impl Default for FillOptions {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            style: peniko::Fill::NonZero,
            transform: None,
            brush: Brush::Solid(peniko::color::palette::css::BLUE),
        }
    }
}

impl BezPath {
    pub fn set_stroke_width(&mut self, width: f32) -> &mut Self {
        self.stroke.style.width = width as f64;
        self
    }
    pub fn set_stroke_color(&mut self, color: impl Into<LinearRgba>) -> &mut Self {
        let color = color.into();

        self.stroke.brush = peniko::Brush::Solid(AlphaColor::new([
            color.red,
            color.green,
            color.blue,
            self.stroke.opacity,
        ]));
        self
    }
    pub fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.stroke.set_opacity(opacity);
        self
    }
    pub fn set_color(&mut self, color: impl Into<LinearRgba> + Copy) -> &mut Self {
        self.set_stroke_color(color);
        self.set_fill_color(color);
        self
    }
    pub fn set_fill_color(&mut self, color: impl Into<LinearRgba>) -> &mut Self {
        let color = color.into();
        self.fill.brush = peniko::Brush::Solid(AlphaColor::new([
            color.red,
            color.green,
            color.blue,
            self.fill.opacity,
        ]));
        self
    }
    pub fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill.set_opacity(opacity);
        self
    }
    // transforms
    pub fn shift(&mut self, shift: (f32, f32)) -> &mut Self {
        self.inner
            .apply_affine(kurbo::Affine::translate((shift.0 as f64, shift.1 as f64)));
        self
    }
    pub fn rotate(&mut self, angle: f32) -> &mut Self {
        self.inner.apply_affine(kurbo::Affine::rotate(angle as f64));
        self
    }
    pub fn scale(&mut self, scale: f32) -> &mut Self {
        self.inner.apply_affine(kurbo::Affine::scale(scale as f64));
        self
    }
}

impl Entity for BezPath {
    type Renderer = CanvasCamera;

    fn tick(&mut self, _dt: f32) {}
    fn extract(&mut self) {}
    fn prepare(&mut self, _ctx: &crate::context::RanimContext) {}
    fn render(&mut self, _ctx: &mut crate::context::RanimContext, renderer: &mut Self::Renderer) {
        renderer.vello_scene.fill(
            self.fill.style,
            kurbo::Affine::IDENTITY,
            &self.fill.brush,
            self.fill.transform,
            &self.inner,
        );
        renderer.vello_scene.stroke(
            &self.stroke.style,
            kurbo::Affine::IDENTITY,
            &self.stroke.brush,
            self.stroke.transform,
            &self.inner,
        );
    }
}

impl Opacity for BezPath {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.set_stroke_opacity(opacity);
        self.set_fill_opacity(opacity);
        self
    }
}