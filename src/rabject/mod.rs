#[deprecated = "Use the new render system based on SDF instead"]
pub mod rabject3d;
// pub mod group;

use std::{fmt::Debug, marker::PhantomData, ops::Deref};

use glam::{ivec3, vec3, IVec3, Vec3};

use crate::{
    context::WgpuContext,
    utils::{Id, RenderResourceStorage},
};

/// A render resource.
pub trait RenderResource {
    fn new(ctx: &WgpuContext) -> Self
    where
        Self: Sized;
}

pub trait Vertex: bytemuck::Pod + bytemuck::Zeroable + Clone + Debug {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

/// Blueprints are the data structures that are used to create [`Rabject`]s
pub trait Blueprint<T> {
    fn build(self) -> T;
}

pub struct RabjectId<R: Rabject>(Id, PhantomData<R>);

impl<R: Rabject> Debug for RabjectId<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RabjectId({:?})", self.0)
    }
}

impl<R: Rabject> Copy for RabjectId<R> {}

impl<R: Rabject> Clone for RabjectId<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R: Rabject> RabjectId<R> {
    pub fn from_id(id: Id) -> Self {
        Self(id, PhantomData)
    }
}

impl<R: Rabject> Deref for RabjectId<R> {
    type Target = Id;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The rabject is the basic object in Ranim.
///
/// ## RenderData
/// The [`Rabject::RenderData`] is the data that is extracted from the rabject and used to initialize/update the render resource.
///
/// ## RenderResource
/// The [`Rabject::RenderResource`] is the resource that is used to render the rabject.
pub trait Rabject {
    type ExtractData;
    type RenderResource: Primitive<Data = Self::ExtractData>;

    fn extract(&self) -> Self::ExtractData;
}

pub trait Primitive {
    type Data;
    fn init(wgpu_ctx: &WgpuContext, data: &Self::Data) -> Self;
    fn update(&mut self, wgpu_ctx: &WgpuContext, data: &Self::Data);
    fn render(
        &self,
        wgpu_ctx: &WgpuContext,
        pipelines: &mut RenderResourceStorage,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
        depth_stencil_view: &wgpu::TextureView,
        uniforms_bind_group: &wgpu::BindGroup,
    );
}

pub trait Updatable {
    fn update_from(&mut self, other: &Self);
}

impl<T: Clone> Updatable for T {
    fn update_from(&mut self, other: &Self) {
        *self = other.clone();
    }
}

/// An empty implementation, for the case that some rabject doesn't need to be rendered (but why?)
impl Primitive for () {
    type Data = ();
    fn init(_wgpu_ctx: &WgpuContext, _data: &Self::Data) -> Self {}
    fn update(&mut self, _wgpu_ctx: &WgpuContext, _data: &Self::Data) {}
    fn render(
        &self,
        _wgpu_ctx: &WgpuContext,
        _pipelines: &mut RenderResourceStorage,
        _multisample_view: &wgpu::TextureView,
        _target_view: &wgpu::TextureView,
        _depth_view: &wgpu::TextureView,
        _uniforms_bind_group: &wgpu::BindGroup,
    ) {
    }
}

