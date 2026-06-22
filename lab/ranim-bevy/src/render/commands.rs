use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::{SetMeshViewBindGroup, SetMeshViewBindingArrayBindGroup},
    render::render_phase::{
        PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
    },
};

use super::gpu::RanimVItemGpuBuffers;

pub(crate) type DrawRanimVItem = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetRanimVItemBindGroup<2>,
    DrawRanimVItemQuad,
);

pub(crate) struct SetRanimVItemBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetRanimVItemBindGroup<I> {
    type Param = SRes<RanimVItemGpuBuffers>;
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        _entity: Option<()>,
        buffers: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let buffers = buffers.into_inner();
        let Some(bind_group) = &buffers.bind_group else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub(crate) struct DrawRanimVItemQuad;

impl<P: PhaseItem> RenderCommand<P> for DrawRanimVItemQuad {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _entity: Option<()>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.draw(0..4, item.batch_range().clone());
        RenderCommandResult::Success
    }
}
