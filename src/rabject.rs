pub mod vgroup;
pub mod vmobject;

use std::{fmt::Debug, marker::PhantomData, ops::Deref};

use crate::{
    utils::{Id, RenderResourceStorage},
    WgpuContext,
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
pub trait Blueprint<T: Rabject> {
    fn build(self) -> T;
}

// pub trait RenderInstance<T: Rabject> {
//     /// Used to initialize the render resource when the rabject is extracted
//     fn init(ctx: &mut RanimContext, rabject: &T) -> Self;

//     fn update(&mut self, ctx: &mut RanimContext, rabject: &RabjectWithId<T>);
// }

// pub trait RabjectId {
//     type Rabject: Rabject;
//     fn from_id(id: Id) -> Self;
//     fn to_id(&self) -> Id;
// }

#[derive(Debug)]
pub struct RabjectId<R: Rabject>(Id, PhantomData<R>);

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
/// ## Id
/// The [`Rabject::Id`] is a type to identify the rabject. It can be a simple wrapper of [`Id`] like this:
/// ```rust
/// struct MyRabjectId(Id);
/// impl RabjectId for MyRabjectId {
///     type Rabject = MyRabject;
///     fn from_id(id: Id) -> Self { Self(id) }
///     fn to_id(self) -> Id { self.0 }
/// }
/// ```
///
/// The reason it exist is just to make the rabject management functions of [`Scene`] support type inference.
///
/// ## RenderData
/// The [`Rabject::RenderData`] is the data that is extracted from the rabject and used to initialize/update the render resource.
///
/// ## RenderResource
/// The [`Rabject::RenderResource`] is the resource that is used to render the rabject.
pub trait Rabject {
    type RenderData: Default;
    type RenderResource: Primitive;

    fn extract(&self) -> Self::RenderData;
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
        depth_view: &wgpu::TextureView,
        uniforms_bind_group: &wgpu::BindGroup,
    );
}

/// An empty implementation, for the case that some rabject doesn't need to be rendered (but why?)
impl Primitive for () {
    type Data = ();
    fn init(_wgpu_ctx: &WgpuContext, _data: &Self::Data) -> Self {
        ()
    }
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
