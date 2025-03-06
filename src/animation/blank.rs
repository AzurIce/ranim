// use crate::render::{RenderTextures, Renderable, StaticRenderable};

// pub struct Blank;

// impl Eval<ItemData> for Blank {
//     fn eval_alpha(&self, alpha: f32) -> EvalResult<ItemData> {
//         EvalResult::None
//     }
// }

// impl Renderable for Blank {
//     fn render(
//         &self,
//         _ctx: &crate::context::WgpuContext,
//         _render_instances: &mut crate::render::primitives::RenderInstances,
//         _pipelines: &mut crate::utils::PipelinesStorage,
//         _encoder: &mut wgpu::CommandEncoder,
//         _uniforms_bind_group: &wgpu::BindGroup,
//         _render_textures: &RenderTextures,
//     ) {
//         // DO NOTHING
//     }
// }

// impl StaticRenderable for Blank {
//     fn prepare(
//         &self,
//         _ctx: &crate::context::WgpuContext,
//         _render_instances: &mut crate::render::primitives::RenderInstances,
//     ) {
//         // DO NOTHING
//     }
// }
