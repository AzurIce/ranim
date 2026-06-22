use ranim_core::{CameraFrame, VItem, store::CoreItemStore};

/// Fill a [`CoreItemStore`] from Ranim VItems.
pub fn collect_vitems_into_store(items: impl IntoIterator<Item = VItem>) -> CoreItemStore {
    let mut store = CoreItemStore::new();
    store.camera_frames.push(CameraFrame::default());
    store.vitems.extend(items);
    store
}
