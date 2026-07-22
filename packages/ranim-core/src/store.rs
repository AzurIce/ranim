use crate::{
    core_item::{CoreItem, mesh_item::MeshItem, vitem::VItem},
    prelude::CameraFrame,
};

/// A store of [`CoreItem`]s.
#[derive(Default, Clone)]
pub struct CoreItemStore {
    /// Id of [`CameraFrame`]s
    pub camera_frame_ids: Vec<(usize, usize)>,
    /// [`CameraFrame`]s
    pub camera_frames: Vec<CameraFrame>,

    /// Id of [`VItem`]s
    pub vitem_ids: Vec<(usize, usize)>,
    /// [`VItem`]s
    pub vitems: Vec<VItem>,

    /// Id of [`MeshItem`]s
    pub mesh_item_ids: Vec<(usize, usize)>,
    /// [`MeshItem`]s
    pub mesh_items: Vec<MeshItem>,
}

impl CoreItemStore {
    /// Create an empty store
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the inner store with the given iterator
    pub fn update(&mut self, items: impl Iterator<Item = ((usize, usize), CoreItem)>) {
        self.camera_frame_ids.clear();
        self.camera_frames.clear();

        self.vitem_ids.clear();
        self.vitems.clear();

        self.mesh_item_ids.clear();
        self.mesh_items.clear();
        for (id, item) in items {
            match item {
                CoreItem::CameraFrame(x) => {
                    self.camera_frame_ids.push(id);
                    self.camera_frames.push(x);
                }
                CoreItem::VItem(x) => {
                    self.vitem_ids.push(id);
                    self.vitems.push(x);
                }
                CoreItem::MeshItem(x) => {
                    self.mesh_item_ids.push(id);
                    self.mesh_items.push(x);
                }
            }
        }
    }
}
