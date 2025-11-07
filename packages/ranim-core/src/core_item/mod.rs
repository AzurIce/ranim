//! Core items of Ranim.
//! 
//! [`CoreItem`]s are the fundamental items of Ranim. All other Items are built upon them.
//! 
//! Currently, there are two types of [`CoreItem`]s:
//! - [`CameraFrame`]: The camera frame.
//! - [`VItemPrimitive`]: The vitem primitive.
use crate::core_item::{camera_frame::CameraFrame, vitem::VItemPrimitive};

/// Camera frame
pub mod camera_frame;
/// Vitem
pub mod vitem;

/// The core ranim builtin items
#[derive(Debug, Clone, PartialEq)]
pub enum CoreItem {
    /// [`CameraFrame`]
    CameraFrame(CameraFrame),
    /// [`VItemPrimitive`]
    VItemPrimitive(VItemPrimitive),
}
