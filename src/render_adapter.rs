//! Adapters from ranim core data to ranim-render input data.

use ranim_core::{CameraFrame, glam::Vec2, store::CoreItemStore};
use ranim_render::scene::{MeshRenderData, RenderScene, VItemRenderData, ViewData};

/// Extension methods for filling a [`RenderScene`] from ranim core data.
pub trait RenderSceneCoreExt {
    /// Replace this render scene with the render data extracted from a core item store.
    fn update_from_core_store(&mut self, store: &CoreItemStore, width: u32, height: u32);
}

impl RenderSceneCoreExt for RenderScene {
    fn update_from_core_store(&mut self, store: &CoreItemStore, width: u32, height: u32) {
        self.reset();

        let camera = store.camera_frames.first().cloned().unwrap_or_default();
        self.view = view_data_from_camera_frame(&camera, width, height);

        self.vitems.extend(store.vitems.iter().map(|vitem| {
            let points = vitem.get_render_points();
            let attr_count = points.len().div_ceil(2);
            VItemRenderData {
                points,
                normal: vitem.normal.map(|n| n.as_vec3()),
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
        }));

        self.meshes
            .extend(store.mesh_items.iter().map(|mesh| MeshRenderData {
                points: mesh.points.to_vec(),
                indices: mesh.triangle_indices.clone(),
                transform: mesh.transform,
                vertex_colors: mesh.vertex_colors.iter().map(|rgba| rgba.0).collect(),
                vertex_normals: mesh.vertex_normals.to_vec(),
            }));
    }
}

/// Convert a core camera frame into render-side view data.
pub fn view_data_from_camera_frame(
    camera_frame: &CameraFrame,
    width: u32,
    height: u32,
) -> ViewData {
    let ratio = width as f64 / height as f64;
    ViewData {
        proj_mat: camera_frame.projection_matrix(ratio).as_mat4(),
        view_mat: camera_frame.view_matrix().as_mat4(),
        half_frame_size: Vec2::new(
            (camera_frame.frame_height * ratio) as f32 / 2.0,
            camera_frame.frame_height as f32 / 2.0,
        ),
    }
}
