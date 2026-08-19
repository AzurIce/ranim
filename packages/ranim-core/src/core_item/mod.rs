//! Core items of Ranim.
//!
//! [`crate::core_item::CoreItem`]s are the fundamental items of Ranim. All other Items are built upon them.
//!
//! Currently, there are three types of [`crate::core_item::CoreItem`]s:
//! - [`crate::core_item::camera_frame::CameraFrame`]: The camera frame.
//! - [`crate::core_item::vitem::VItem`]: The vitem primitive.
//! - [`crate::core_item::mesh_item::MeshItem`]: The mesh primitive.
use std::any::Any;

use dyn_clone::DynClone;

use crate::{
    Extract,
    core_item::{camera_frame::CameraFrame, mesh_item::MeshItem, vitem::VItem},
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
