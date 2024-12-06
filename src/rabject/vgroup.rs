use crate::{utils::RenderResourceStorage, WgpuContext};

use super::{
    vmobject::{
        primitive::{ExtractedVMobject, VMobjectPrimitive},
        VMobject,
    },
    Primitive, Rabject,
};

pub struct VGroup {
    pub(crate) children: Vec<VMobject>,
}

impl VGroup {
    pub fn new(children: Vec<VMobject>) -> Self {
        Self { children }
    }
}

pub struct VGroupPrimitive {
    children: Vec<VMobjectPrimitive>,
}

impl Primitive for VGroupPrimitive {
    type Data = Vec<ExtractedVMobject>;
    fn init(wgpu_ctx: &WgpuContext, data: &Self::Data) -> Self {
        let children = data
            .iter()
            .map(|e| VMobjectPrimitive::init(wgpu_ctx, &e))
            .collect::<Vec<_>>();
        Self { children }
    }

    fn update(&mut self, wgpu_ctx: &WgpuContext, data: &Self::Data) {
        for (i, child) in self.children.iter_mut().enumerate() {
            child.update(wgpu_ctx, &data[i]);
        }
    }

    fn render(
        &self,
        wgpu_ctx: &WgpuContext,
        pipelines: &mut RenderResourceStorage,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        uniforms_bind_group: &wgpu::BindGroup,
    ) {
        for child in self.children.iter() {
            child.render(
                wgpu_ctx,
                pipelines,
                multisample_view,
                target_view,
                depth_view,
                uniforms_bind_group,
            );
        }
    }
}

impl Rabject for VGroup {
    type RenderData = Vec<ExtractedVMobject>;
    type RenderResource = VGroupPrimitive;

    fn extract(&self) -> Self::RenderData {
        self.children.iter().map(|e| e.extract()).collect()
    }
}
