
use crate::{
    Extract,
    primitives::{camera_frame::CameraFrame, vitem::VItemPrimitive},
};

pub mod camera_frame;
pub mod vitem;

pub trait Primitive {
    fn build_primitives<T: IntoIterator<Item = Self>>(iter: T) -> Primitives;
}

impl<T: Primitive + Clone> Extract for T {
    type Target = Self;
    fn extract(&self) -> Vec<Self::Target> {
        vec![self.clone()]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Primitives {
    CameraFrame(Vec<CameraFrame>),
    VItemPrimitive(Vec<VItemPrimitive>),
}
