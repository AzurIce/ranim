//! Core items of Ranim.
//!
//! [`CoreItem`]s are the fundamental items of Ranim. All other Items are built upon them.
//!
//! Currently, there are two types of [`CoreItem`]s:
//! - [`CameraFrame`]: The camera frame.
//! - [`VItem`]: The vitem primitive.
use std::any::Any;

use dyn_clone::DynClone;

use crate::{
    Extract,
    core_item::{camera_frame::CameraFrame, mesh_item::MeshItem, vitem::VItem},
    traits::Interpolatable,
};

/// Camera frame
pub mod camera_frame;
/// MeshItem
pub mod mesh_item;
/// Vitem
pub mod vitem;

/// The core ranim builtin items
#[derive(Debug, Clone, PartialEq)]
pub enum CoreItem {
    /// [`CameraFrame`]
    CameraFrame(CameraFrame),
    /// [`VItem`]
    VItem(VItem),
    /// [`MeshItem`]
    MeshItem(MeshItem),
}

/// The item that can be extracted to [`CoreItem`]s
pub trait AnyExtractCoreItem: Any + Extract<Target = CoreItem> + DynClone {}
impl<T: Extract<Target = CoreItem> + Any + DynClone> AnyExtractCoreItem for T {}

dyn_clone::clone_trait_object!(AnyExtractCoreItem);

/// A dynamic item, basically type erased [`AnyExtractCoreItem`]
#[derive(Clone)]
pub struct DynItem(pub Box<dyn AnyExtractCoreItem>);

impl Extract for DynItem {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        self.0.extract_into(buf);
    }
}

impl<T: Into<CoreItem> + Clone> Extract for T {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        buf.push(self.clone().into());
    }
}

impl<T: Extract<Target = CoreItem>> Extract for Vec<T> {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        for item in self {
            item.extract_into(buf);
        }
    }
}

impl Interpolatable for CoreItem {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        match (self, target) {
            (CoreItem::CameraFrame(a), CoreItem::CameraFrame(b)) => {
                CoreItem::CameraFrame(a.lerp(b, t))
            }
            (CoreItem::VItem(a), CoreItem::VItem(b)) => CoreItem::VItem(a.lerp(b, t)),
            (CoreItem::MeshItem(a), CoreItem::MeshItem(b)) => CoreItem::MeshItem(a.lerp(b, t)),
            // Mismatched variants: snap at t=1
            _ => {
                if t < 1.0 {
                    self.clone()
                } else {
                    target.clone()
                }
            }
        }
    }
}
