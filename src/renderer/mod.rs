use std::fmt::Debug;

use crate::{rabject::Rabject, RenderResourceStorage, WgpuContext};

pub mod vmobject;

/// A render resource.
pub trait RenderResource {
    fn new(ctx: &WgpuContext) -> Self
    where
        Self: Sized;
}

pub trait Vertex: bytemuck::Pod + bytemuck::Zeroable + Clone + Debug {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

// pub trait Renderer<R: Rabject> {
//     fn render(
//         &self,
//         wgpu_ctx: &WgpuContext,
//         pipelines: &mut RenderResourceStorage,
//         render_instances: &[&R::RenderInstance],
//         multisample_view: &wgpu::TextureView,
//         target_view: &wgpu::TextureView,
//         depth_view: &wgpu::TextureView,
//         uniforms_bind_group: &wgpu::BindGroup,
//     );
// }
