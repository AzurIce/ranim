use crate::render::RenderTextures;

use super::{
    Primitive, Renderable,
    vitem::{VItemPrimitive, VItemPrimitiveData},
};

#[derive(Default)]
pub struct SvgItemPrimitive {
    pub(crate) vitem_primitives: Vec<VItemPrimitive>,
}

pub struct SvgItemPrimitiveData {
    pub vitem_datas: Vec<VItemPrimitiveData>,
}

impl Primitive for SvgItemPrimitive {
    type Data = SvgItemPrimitiveData;

    fn init(ctx: &crate::context::WgpuContext, data: &Self::Data) -> Self {
        let vitem_primitives = data
            .vitem_datas
            .iter()
            .map(|vitem_primitive| VItemPrimitive::init(ctx, vitem_primitive))
            .collect();
        Self { vitem_primitives }
    }
    fn update(&mut self, ctx: &crate::context::WgpuContext, data: &Self::Data) {
        self.vitem_primitives
            .iter_mut()
            .zip(data.vitem_datas.iter())
            .for_each(|(vitem_primitive, vitem_data)| {
                vitem_primitive.update(ctx, vitem_data);
            });
    }
}

impl Renderable for SvgItemPrimitive {
    fn encode_render_pass_command<'a>(&self, rpass: &mut wgpu::RenderPass<'a>) {
        self.vitem_primitives.iter().for_each(|vimte_primitive| {
            vimte_primitive.encode_render_pass_command(rpass);
        })
    }
    fn encode_compute_pass_command<'a>(&self, cpass: &mut wgpu::ComputePass<'a>) {
        self.vitem_primitives.iter().for_each(|vimte_primitive| {
            vimte_primitive.encode_compute_pass_command(cpass);
        })
    }
    fn encode_render_command(
        &self,
        ctx: &crate::context::WgpuContext,
        pipelines: &mut crate::render::PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        #[cfg(feature = "profiling")] profiler: &mut wgpu_profiler::GpuProfiler,
    ) {
        // trace!("SvgItemPrimitive encode_render_command vitem_primitives: {}", self.vitem_primitives.len());
        self.vitem_primitives.iter().for_each(|vimte_primitive| {
            vimte_primitive.encode_render_command(
                ctx,
                pipelines,
                encoder,
                uniforms_bind_group,
                render_textures,
                #[cfg(feature = "profiling")]
                profiler,
            );
        });
    }
}
