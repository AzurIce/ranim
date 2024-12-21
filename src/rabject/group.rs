use crate::{
    context::WgpuContext,
    prelude::{Alignable, Interpolatable, Opacity},
    utils::RenderResourceStorage,
};

use super::{Primitive, Rabject};

/// A group of same type [`Rabject`]s
#[derive(Clone)]
pub struct Group<R: Rabject> {
    pub(crate) children: Vec<R>,
}

impl<R: Rabject> Group<R> {
    pub fn new(children: Vec<R>) -> Self {
        Self { children }
    }
}

pub struct GroupPrimitive<R: Primitive> {
    children: Vec<R>,
}

impl<R: Primitive> Primitive for GroupPrimitive<R> {
    type Data = Vec<R::Data>;
    fn init(wgpu_ctx: &WgpuContext, data: &Self::Data) -> Self {
        let children = data
            .iter()
            .map(|e| R::init(wgpu_ctx, e))
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

impl<R: Rabject> Rabject for Group<R> {
    type RenderData = Vec<R::RenderData>;
    type RenderResource = GroupPrimitive<R::RenderResource>;

    fn extract(&self) -> Self::RenderData {
        self.children.iter().map(|e| e.extract()).collect()
    }
}

impl<R: Rabject + Alignable + Default + Clone> Alignable for Group<R> {
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
            self.children.resize(max_len, R::default());
        } else {
            other.children.resize(max_len, R::default());
        }

        self.children
            .iter_mut()
            .zip(other.children.iter_mut())
            .for_each(|(a, b)| {
                a.align_with(b);
            });
    }
}

impl<R: Rabject + Interpolatable> Interpolatable for Group<R> {
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

impl<R: Rabject + Opacity> Opacity for Group<R> {
    fn set_opacity(&mut self, opacity: f32) {
        self.children.iter_mut().for_each(|e| {
            e.set_opacity(opacity);
        });
    }
}
