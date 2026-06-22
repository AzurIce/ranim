use bevy::{
    prelude::*,
    render::sync_world::{MainEntity, MainEntityHashMap},
};
use ranim_core::VItem;
use ranim_render::scene::VItemRenderData;

#[derive(Clone, Debug)]
pub(crate) struct RenderRanimVItem {
    pub(crate) main_entity: MainEntity,
    pub(crate) item: VItemRenderData,
    pub(crate) world_from_local: Mat4,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub(crate) struct RenderRanimVItems(MainEntityHashMap<RenderRanimVItem>);

pub(crate) fn vitem_world_center(item: &RenderRanimVItem) -> Vec3 {
    if item.item.points.is_empty() {
        return item.world_from_local.transform_point3(Vec3::ZERO);
    }

    let sum = item.item.points.iter().fold(Vec3::ZERO, |sum, point| {
        sum + Vec3::new(point.x, point.y, point.z)
    });
    let local_center = sum / item.item.points.len() as f32;
    item.world_from_local.transform_point3(local_center)
}

pub(crate) fn vitem_render_data(vitem: &VItem) -> VItemRenderData {
    let points = vitem.get_render_points();
    let attr_count = points.len().div_ceil(2);

    VItemRenderData {
        points,
        normal: vitem.normal.map(|normal| normal.as_vec3()),
        fill_rgbas: vitem
            .fill_rgbas
            .resize_by_sample(attr_count)
            .into_iter()
            .map(|rgba| rgba.0)
            .collect(),
        stroke_rgbas: vitem
            .stroke_rgbas
            .resize_by_sample(attr_count)
            .into_iter()
            .map(|rgba| rgba.0)
            .collect(),
        stroke_widths: vitem
            .stroke_widths
            .resize_by_sample(attr_count)
            .into_iter()
            .map(|width| width.0)
            .collect(),
    }
}
