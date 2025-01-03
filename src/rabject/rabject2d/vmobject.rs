use bevy_color::LinearRgba;
use glam::{vec2, Vec2};
use vello::kurbo::{self, Affine, PathEl};

use crate::{
    prelude::{Alignable, Interpolatable, Opacity},
    scene::{canvas::camera::CanvasCamera, Entity},
    utils::affine_from_vec,
};

use super::bez_path::BezPath;

pub mod geometry;
pub mod svg;

#[derive(Clone, Debug)]
pub struct VMobject {
    subpaths: Vec<BezPath>,
}

impl VMobject {
    pub fn shift(&mut self, offset: Vec2) -> &mut Self {
        self.apply_affine(Affine::translate((offset.x as f64, offset.y as f64)));
        self
    }
    pub fn rotate(&mut self, angle: f32) -> &mut Self {
        self.apply_affine(Affine::rotate(angle as f64));
        self
    }
    pub fn set_color(&mut self, color: impl Into<LinearRgba> + Copy) -> &mut Self {
        self.subpaths.iter_mut().for_each(|p| {
            p.set_color(color);
        });
        self
    }
    pub fn set_fill_color(&mut self, color: impl Into<LinearRgba> + Copy) -> &mut Self {
        self.subpaths.iter_mut().for_each(|p| {
            p.set_fill_color(color);
        });
        self
    }
    pub fn set_stroke_color(&mut self, color: impl Into<LinearRgba> + Copy) -> &mut Self {
        self.subpaths.iter_mut().for_each(|p| {
            p.set_stroke_color(color);
        });
        self
    }
    pub fn set_stroke_width(&mut self, width: f32) -> &mut Self {
        self.subpaths.iter_mut().for_each(|p| {
            p.set_stroke_width(width);
        });
        self
    }
    pub fn set_fill_alpha(&mut self, alpha: f32) -> &mut Self {
        self.subpaths.iter_mut().for_each(|p| {
            p.set_fill_opacity(alpha);
        });
        self
    }
}

impl VMobject {
    pub fn get_start_and_end(&self) -> Option<(Vec2, Vec2)> {
        let start = self
            .subpaths
            .first()
            .and_then(|p| {
                p.elements().first().map(|e| match e {
                    PathEl::MoveTo(p) => p,
                    _ => unreachable!("a BezPath should start with MoveTo"),
                })
            })
            .map(|p| vec2(p.x as f32, p.y as f32));
        let end = self
            .subpaths
            .last()
            .and_then(|p| {
                p.elements()
                    .iter()
                    .rfind(|e| !matches!(e, PathEl::ClosePath))
                    .map(|e| match e {
                        PathEl::LineTo(p) => p,
                        PathEl::QuadTo(_, p) => p,
                        PathEl::CurveTo(_, _, p) => p,
                        _ => unreachable!("a BezPath should not end with MoveTo"),
                    })
            })
            .map(|p| vec2(p.x as f32, p.y as f32));
        start.zip(end)
    }

    pub fn put_start_and_end_on(&mut self, start: Vec2, end: Vec2) {
        let (cur_start, cur_end) = self.get_start_and_end().unwrap();
        // println!("cur_start: {:?}, cur_end: {:?}, start: {:?}, end: {:?}", cur_start, cur_end, start, end);

        let cur_vec = cur_end - cur_start;
        let target_vec = end - start;

        let transform = Affine::translate((start.x as f64, start.y as f64))
            * affine_from_vec(cur_vec, target_vec)
            * Affine::translate((-cur_start.x as f64, -cur_start.y as f64));
        self.apply_affine(transform);
    }
}

impl VMobject {
    pub fn empty() -> Self {
        VMobject { subpaths: vec![] }
    }
    pub fn new(subpaths: Vec<BezPath>) -> Self {
        Self { subpaths }
    }
    pub fn apply_affine(&mut self, affine: kurbo::Affine) {
        for path in self.subpaths.iter_mut() {
            path.apply_affine(affine);
        }
    }
    pub fn push(&mut self, path: BezPath) {
        self.subpaths.push(path);
    }
    pub fn extend(&mut self, paths: Vec<BezPath>) {
        self.subpaths.extend(paths);
    }
}

impl Entity for VMobject {
    type Renderer = CanvasCamera;
    fn tick(&mut self, _dt: f32) {}
    fn extract(&mut self) {}
    fn prepare(&mut self, _ctx: &crate::context::RanimContext) {}
    fn render(&mut self, _ctx: &mut crate::context::RanimContext, renderer: &mut Self::Renderer) {
        for path in self.subpaths.iter_mut() {
            path.render(_ctx, renderer);
        }
    }
}

impl Alignable for VMobject {
    fn is_aligned(&self, other: &Self) -> bool {
        self.subpaths.len() == other.subpaths.len()
            && self
                .subpaths
                .iter()
                .zip(other.subpaths.iter())
                .all(|(a, b)| a.is_aligned(b))
    }
    fn align_with(&mut self, other: &mut Self) {
        let len = self.subpaths.len().max(other.subpaths.len());
        if self.subpaths.len() < len {
            self.subpaths
                .resize(len, self.subpaths.last().cloned().unwrap());
        } else {
            other
                .subpaths
                .resize(len, other.subpaths.last().cloned().unwrap());
        }

        self.subpaths
            .iter_mut()
            .zip(other.subpaths.iter_mut())
            .for_each(|(a, b)| a.align_with(b));
    }
}

impl Interpolatable for VMobject {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        let subpaths = self
            .subpaths
            .iter()
            .zip(target.subpaths.iter())
            .map(|(a, b)| a.lerp(b, t))
            .collect();
        VMobject { subpaths }
    }
}

impl Opacity for VMobject {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.subpaths.iter_mut().for_each(|p| {
            p.set_opacity(opacity);
        });
        self
    }
}
