//! Core items of Ranim.
//!
//! [`crate::core_item::CoreItem`]s are the fundamental items of Ranim. All other Items are built upon them.
//!
//! Currently, there are three types of [`crate::core_item::CoreItem`]s:
//! - [`crate::core_item::camera_frame::CameraFrame`]: The camera frame.
//! - [`crate::core_item::vitem::VItem`]: The vitem primitive.
//! - [`crate::core_item::mesh_item::MeshItem`]: The mesh primitive.
use std::alloc::{Allocator, Global};
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
/// Transformed
pub mod transformed;
/// Vitem
pub mod vitem;

/// The core ranim builtin items, with vectors living in allocator `A`
/// (default [`Global`]). See [`VItem::clone_in`][vitem::VItem].
#[derive(Debug, Clone)]
pub enum CoreItem<A: Allocator = Global> {
    /// [`CameraFrame`]
    CameraFrame(CameraFrame),
    /// [`VItem`]
    VItem(VItem<A>),
    /// [`MeshItem`]
    MeshItem(MeshItem<A>),
}

impl PartialEq for CoreItem {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::CameraFrame(a), Self::CameraFrame(b)) => a == b,
            (Self::VItem(a), Self::VItem(b)) => a == b,
            (Self::MeshItem(a), Self::MeshItem(b)) => a == b,
            _ => false,
        }
    }
}

impl<A: Allocator> CoreItem<A> {
    /// Apply a local-to-world transform to this item.
    ///
    /// - [`CameraFrame`]: transforms `pos` as a point, re-normalizes `up`/`facing` as vectors.
    /// - [`VItem`]: left-multiplies its `transform` matrix (points untouched).
    /// - [`MeshItem`]: left-multiplies its `transform` matrix (vertices untouched).
    pub fn apply_transform(&mut self, transform: &glam::DAffine3) {
        match self {
            CoreItem::CameraFrame(cam) => {
                cam.pos = transform.transform_point3(cam.pos);
                cam.up = transform.transform_vector3(cam.up).normalize();
                cam.facing = transform.transform_vector3(cam.facing).normalize();
            }
            CoreItem::VItem(item) => {
                item.transform = glam::DMat4::from(*transform).as_mat4() * item.transform;
            }
            CoreItem::MeshItem(item) => {
                item.transform = glam::DMat4::from(*transform).as_mat4() * item.transform;
            }
        }
    }

    /// Move into `alloc`'s memory, producing an allocator-owned item.
    ///
    /// Element data is copied into the new allocator; no global allocation
    /// is added by this call itself.
    pub fn into_arena<A2: Allocator + Clone>(self, alloc: A2) -> CoreItem<A2> {
        match self {
            CoreItem::CameraFrame(item) => CoreItem::CameraFrame(item),
            CoreItem::VItem(item) => CoreItem::VItem(item.into_arena(alloc)),
            CoreItem::MeshItem(item) => CoreItem::MeshItem(item.into_arena(alloc)),
        }
    }

    /// Move back into the global heap (the boundary the render world —
    /// bevy components — lives on).
    pub fn into_global(self) -> CoreItem {
        self.into_arena(Global)
    }

    /// Copy into `alloc`'s memory, producing an allocator-owned item.
    pub fn clone_in<A2: Allocator + Clone>(&self, alloc: A2) -> CoreItem<A2> {
        match self {
            CoreItem::CameraFrame(item) => CoreItem::CameraFrame(item.clone()),
            CoreItem::VItem(item) => CoreItem::VItem(item.clone_in(alloc)),
            CoreItem::MeshItem(item) => CoreItem::MeshItem(item.clone_in(alloc)),
        }
    }
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
