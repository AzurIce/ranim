pub mod group;
pub mod vmobject;

use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use crate::{
    // renderer::{RenderResource, Renderer},
    scene::{Entity, Scene},
    utils::{Id, RenderResourceStorage},
    RanimContext,
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

pub trait RabjectId {
    fn from_id(id: Id) -> Self;
    fn to_id(&self) -> Id;
}

/// The rabject is the basic object in Ranim.
///
/// ## Id
/// The [`Rabject::Id`] is a type to identify the rabject. It can be a simple wrapper of [`Id`] like this:
/// ```rust
/// struct MyRabjectId(Id);
/// impl RabjectId for MyRabjectId {
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
pub trait Rabject: 'static + Sized {
    type Id;
    type Data;
    type RenderData: Default;
    type RenderResource: Primitive;

    fn insert_to_scene(self, scene: &mut Scene) -> Self::Id;

    fn remove_from_scene(scene: &mut Scene, id: Self::Id);

    fn extract(&self) -> Self::RenderData {
        Default::default()
    }
}

pub trait Primitive {
    type Data;
    fn init(ctx: &mut RanimContext, data: &Self::Data) -> Self;
    fn update(&mut self, ctx: &mut RanimContext, data: &Self::Data);
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

impl Primitive for () {
    type Data = ();
    fn init(ctx: &mut RanimContext, data: &Self::Data) -> Self {
        ()
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
    }
    fn update(&mut self, ctx: &mut RanimContext, data: &Self::Data) {}
}

// #[derive(Clone)]
// pub struct RabjectWithId<T: Rabject> {
//     id: Id,
//     rabject: T,
// }

// impl<T: Rabject> Deref for RabjectWithId<T> {
//     type Target = T;

//     fn deref(&self) -> &Self::Target {
//         &self.rabject
//     }
// }

// impl<T: Rabject> DerefMut for RabjectWithId<T> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.rabject
//     }
// }

// impl<T: Rabject> From<T> for RabjectWithId<T> {
//     fn from(rabject: T) -> Self {
//         Self {
//             id: Id::new(),
//             rabject,
//         }
//     }
// }

// impl<T: Rabject> RabjectWithId<T> {
//     pub fn id(&self) -> &Id {
//         &self.id
//     }

//     pub fn extract(&self, ctx: &mut RanimContext) -> ExtractedRabjectWithId<T> {
//         ExtractedRabjectWithId {
//             id: self.id,
//             render_resource: T::RenderInstance::init(ctx, &self.rabject),
//         }
//     }
// }

// pub struct ExtractedRabjectWithId<T: Rabject> {
//     id: Id,
//     // rabject: Arc<RwLock<T>>,
//     pub(crate) render_resource: T::RenderInstance,
// }

// impl<T: Rabject> ExtractedRabjectWithId<T> {
//     pub fn id(&self) -> &Id {
//         &self.id
//     }
// }

// impl<T: Rabject> Deref for ExtractedRabjectWithId<T> {
//     type Target = T::RenderInstance;

//     fn deref(&self) -> &Self::Target {
//         &self.render_resource
//     }
// }

// impl<T: Rabject> DerefMut for ExtractedRabjectWithId<T> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.render_resource
//     }
// }
