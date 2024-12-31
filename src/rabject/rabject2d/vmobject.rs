use log::trace;
use vello::kurbo;

use crate::{
    prelude::{Alignable, Interpolatable},
    scene::{canvas::camera::CanvasCamera, Entity},
};

use super::bez_path::BezPath;

#[derive(Clone, Debug)]
pub struct VMobject {
    subpaths: Vec<BezPath>,
}

// impl Default for VMobject {
//     fn default() -> Self {
//         VMobject::Path(BezPath::default())
//     }
// }

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
        println!("self: {}", self.subpaths.len());
        println!("other: {}", other.subpaths.len());

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
