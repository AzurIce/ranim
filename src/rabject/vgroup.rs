use crate::{
    prelude::{Alignable, Interpolatable, Opacity},
    utils::RenderResourceStorage,
    context::WgpuContext,
};

use super::{
    vmobject::{
        primitive::{ExtractedVMobject, VMobjectPrimitive},
        VMobject,
    },
    Primitive, Rabject,
};

#[derive(Clone)]
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
            .map(|e| VMobjectPrimitive::init(wgpu_ctx, e))
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

    fn update_from(&mut self, other: &Self) {
        self.children = other.children.clone();
    }
}

impl Alignable for VGroup {
    fn is_aligned(&self, other: &Self) -> bool {
        self.children.len() == other.children.len()
            && self
                .children
                .iter()
                .zip(other.children.iter())
                .all(|(a, b)| a.is_aligned(b))
    }

    fn align_with(&mut self, other: &mut Self) {
        let max_len = self.children.len().max(other.children.len());
        if self.children.len() < max_len {
            self.children.resize(max_len, VMobject::default());
        } else {
            other.children.resize(max_len, VMobject::default());
        }

        self.children
            .iter_mut()
            .zip(other.children.iter_mut())
            .for_each(|(a, b)| {
                a.align_with(b);
            });
    }
}

impl Interpolatable for VGroup {
    fn lerp(&self, other: &Self, alpha: f32) -> Self {
        Self::new(
            self.children
                .iter()
                .zip(other.children.iter())
                .map(|(a, b)| a.lerp(b, alpha))
                .collect(),
        )
    }
}

impl Opacity for VGroup {
    fn set_opacity(&mut self, opacity: f32) {
        self.children.iter_mut().for_each(|e| {
            e.set_opacity(opacity);
        });
    }
}
