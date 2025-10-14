use crate::{
    Extract,
    primitives::{camera_frame::CameraFrame, vitem::VItemPrimitive},
};

/// Camera frame
pub mod camera_frame;
/// Vitem
pub mod vitem;

/// The most basic building block in ranim.
pub trait Primitive {
    /// Build primitives
    fn build_primitives<T: IntoIterator<Item = Self>>(iter: T) -> Primitives;
}

impl<T: Primitive + Clone> Extract for T {
    type Target = Self;
    fn extract(&self) -> Vec<Self::Target> {
        vec![self.clone()]
    }
}

/// A collection of [`Primitive`]s
#[derive(Debug, Clone, PartialEq)]
pub enum Primitives {
    /// `Vec<CameraFrame>`
    CameraFrame(Vec<CameraFrame>),
    /// `Vec<VItemPrimitive>`
    VItemPrimitive(Vec<VItemPrimitive>),
}

impl Primitives {
    /// This is temporary to convert [`Primitives`] to [`CoreItem`]s
    pub fn boom(self) -> Vec<CoreItem> {
        match self {
            Primitives::CameraFrame(x) => x.into_iter().map(CoreItem::CameraFrame).collect(),
            Primitives::VItemPrimitive(x) => x.into_iter().map(CoreItem::VItemPrimitive).collect(),
        }
    }
}

/// The core ranim builtin items
pub enum CoreItem {
    /// [`CameraFrame`]
    CameraFrame(CameraFrame),
    /// [`VItemPrimitive`]
    VItemPrimitive(VItemPrimitive),
}
