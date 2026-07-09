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
    anchor::{DBounds3, SemanticBounds},
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

impl SemanticBounds for CoreItem {
    fn semantic_bounds(&self) -> DBounds3 {
        match self {
            CoreItem::CameraFrame(item) => item.semantic_bounds(),
            CoreItem::VItem(item) => item.semantic_bounds(),
            CoreItem::MeshItem(item) => item.semantic_bounds(),
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

#[cfg(test)]
mod tests {
    use glam::{Mat4, Vec3};

    use super::*;

    #[test]
    fn core_mesh_semantic_bounds_apply_transform() {
        let mesh = MeshItem {
            points: vec![Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 2.0, 0.0)],
            triangle_indices: Vec::new(),
            transform: Mat4::from_translation(Vec3::new(3.0, 0.0, 0.0))
                * Mat4::from_scale(Vec3::new(2.0, 1.0, 1.0)),
            vertex_colors: Vec::new(),
            vertex_normals: Vec::new(),
        };

        let bounds = CoreItem::MeshItem(mesh).semantic_bounds();

        assert_eq!(bounds.world_min(), glam::dvec3(1.0, 0.0, 0.0));
        assert_eq!(bounds.world_max(), glam::dvec3(5.0, 2.0, 0.0));
    }
}
