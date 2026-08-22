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
    logic::{MaterializeCtx, MaterializeOut},
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

/// A dynamic item: a type-erased [`AnyExtractCoreItem`] carrying its own
/// materialize hook (M2).
///
/// The hook is monomorphized at the single erasure point ([`DynItem::new`]):
/// it downcasts back to the concrete type and upserts the value into the
/// `LogicWorld`. A `DynItem` can therefore materialize itself without the
/// host knowing its type — including items captured into static cells
/// (`hold`, lagged fills), where the concrete type is otherwise lost.
#[derive(Clone)]
pub struct DynItem {
    inner: Box<dyn AnyExtractCoreItem>,
    materialize: fn(Self, &mut MaterializeCtx, u32),
}

/// Materialize hook for `T`, monomorphized at [`DynItem`] construction.
fn materialize_hook<T: MaterializeOut>(item: DynItem, ctx: &mut MaterializeCtx, part: u32) {
    let any: Box<dyn Any> = item.inner;
    let typed = any
        .downcast::<T>()
        .expect("DynItem materialize hook type mismatch");
    typed.materialize(ctx, part);
}

impl DynItem {
    /// Erase a materializable item, capturing its typed materialize hook.
    pub fn new<T: MaterializeOut>(item: T) -> Self {
        Self {
            inner: Box::new(item),
            materialize: materialize_hook::<T>,
        }
    }

    /// Materialize into the `LogicWorld` at the given `part` slot (M2).
    pub(crate) fn materialize(self, ctx: &mut MaterializeCtx, part: u32) {
        (self.materialize)(self, ctx, part);
    }
}

impl Extract for DynItem {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        self.inner.extract_into(buf);
    }
}
