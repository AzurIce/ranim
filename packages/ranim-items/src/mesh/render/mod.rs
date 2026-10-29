//! Render-world data and GPU implementation for mesh items.

mod buffer;
mod pipeline;

use ranim_core::{
    components::rgba::Rgba,
    glam::{Mat4, Vec3},
};

pub(super) fn install(app: &mut bevy_app::App) {
    use bevy_ecs::schedule::IntoScheduleConfigs;
    use ranim_render::{
        extract::ExtractPlugin,
        schedule::{RenderSchedule, ViewPhase},
        utils::WgpuContext,
    };

    use super::{MeshItem, Sphere, Surface};

    let buffer = buffer::MeshItemsBuffer::new(app.world().resource::<WgpuContext>());
    app.insert_resource(buffer).add_plugins((
        ExtractPlugin::<MeshItem>::default(),
        ExtractPlugin::<Sphere>::default(),
        ExtractPlugin::<Surface>::default(),
    ));
    app.add_systems(RenderSchedule::Prepare, pipeline::prepare_mesh_items)
        .add_systems(
            RenderSchedule::View,
            (
                pipeline::depth.in_set(ViewPhase::Depth),
                pipeline::color.in_set(ViewPhase::Color),
            ),
        );
}

/// A primitive for rendering a mesh item.
#[derive(bevy_ecs::component::Component, Debug, Clone, PartialEq)]
pub struct RenderMeshItem {
    /// The vertices of the mesh
    pub points: Vec<Vec3>,
    /// The triangle indices
    pub triangle_indices: Vec<u32>,
    /// The transform matrix
    pub transform: Mat4,
    /// Per-vertex colors
    pub vertex_colors: Vec<Rgba>,
    /// Per-vertex normals for smooth shading.
    /// All-zero (or empty) → shader falls back to flat shading via `dpdx`/`dpdy`.
    pub vertex_normals: Vec<Vec3>,
}

impl Default for RenderMeshItem {
    fn default() -> Self {
        Self {
            points: vec![Vec3::ZERO; 3],
            triangle_indices: vec![0, 1, 2],
            transform: Mat4::IDENTITY,
            vertex_colors: vec![Rgba::default(); 3],
            vertex_normals: vec![Vec3::ZERO; 3],
        }
    }
}
