//! Core items of Ranim.
//!
//! [`CoreItem`]s are the fundamental items of Ranim. All other Items are built upon them.
//!
//! Currently, there are two types of [`CoreItem`]s:
//! - [`CameraFrame`]: The camera frame.
//! - [`VItemPrimitive`]: The vitem primitive.
use std::any::Any;

use dyn_clone::DynClone;

use crate::{
    Extract,
    core_item::{camera_frame::CameraFrame, vitem::VItemPrimitive, vitem_2d::VItem2d},
};

/// Camera frame
pub mod camera_frame;
/// Vitem
pub mod vitem;
/// Vitem in 2d plane
pub mod vitem_2d;

/// The core ranim builtin items
#[derive(Debug, Clone, PartialEq)]
pub enum CoreItem {
    /// [`CameraFrame`]
    CameraFrame(CameraFrame),
    /// [`VItemPrimitive`]
    VItemPrimitive(VItemPrimitive),
    /// [`VItem2d`]
    VItem2D(VItem2d),
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
