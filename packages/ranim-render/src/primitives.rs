/// Primitive for vitem
pub mod vitem;
pub mod vitem2d;

use crate::utils::WgpuContext;

/// The RenderResource encapsules the wgpu resources.
///
/// It has a `Data` type that is used to initialize/update the resource.
pub trait RenderResource {
    /// The type used for [`RenderResource::init`] and [`RenderResource::update`].
    type Data;
    /// Initialize a RenderResource using [`RenderResource::Data`]
    fn init(ctx: &WgpuContext, data: &Self::Data) -> Self;
    /// update a RenderResource using [`RenderResource::Data`]
    fn update(&mut self, ctx: &WgpuContext, data: &Self::Data);
}

/// The Primitive is the basic renderable object in Ranim.
///
/// The Primitive itself is simply the data of the object.
/// A Primitive can generate a corresponding [`Primitive::RenderPacket`],
/// which implements [`RenderResource`]:
/// - [`RenderResource`]: A trait about init or update itself with [`RenderResource::Data`].
pub trait Primitive {
    /// The RenderInstance
    type RenderPacket: RenderResource<Data = Self> + Send + Sync + 'static;
}
