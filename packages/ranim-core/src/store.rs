use crate::{
    prelude::CameraFrame,
    primitives::{CoreItem, vitem::VItemPrimitive},
};

/// A store of [`CoreItem`]s.
#[derive(Default, Clone)]
pub struct CoreItemStore {
    /// Id, CameraFrames
    pub camera_frames: Vec<CameraFrame>,
    /// Id, VItemPrimitive
    pub vitems: Vec<VItemPrimitive>,
}

impl CoreItemStore {
    /// Create an empty store
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the inner store with the given iterator
    pub fn update(&mut self, items: impl Iterator<Item = CoreItem>) {
        self.camera_frames.clear();
        self.vitems.clear();
        for item in items {
            match item {
                CoreItem::CameraFrame(x) => self.camera_frames.push(x),
                CoreItem::VItemPrimitive(x) => self.vitems.push(x),
            }
        }
    }
}
