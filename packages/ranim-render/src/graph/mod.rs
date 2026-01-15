use std::ops::{Deref, DerefMut};

pub mod view;

pub mod clear;
pub use clear::*;

use variadics_please::all_tuples;

use crate::{
    RenderContext,
    primitives::{viewport::ViewportGpuPacket, vitem::VItemRenderInstance},
    resource::Handle,
    utils::collections::{Graph, TypeBinnedVec},
};

slotmap::new_key_type! { pub struct GlobalRenderNodeKey; }
/// Global render graph is something executed globally, which is, NOT per-view.
///
/// For per-view's render graph see [`view::ViewRenderGraph`].
#[derive(Default)]
pub struct GlobalRenderGraph {
    inner: Graph<GlobalRenderNodeKey, Box<dyn AnyGlobalRenderNodeTrait + Send + Sync>>,
}

impl Deref for GlobalRenderGraph {
    type Target = Graph<GlobalRenderNodeKey, Box<dyn AnyGlobalRenderNodeTrait + Send + Sync>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for GlobalRenderGraph {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl GlobalRenderGraph {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert_node(
        &mut self,
        node: impl AnyGlobalRenderNodeTrait + Send + Sync + 'static,
    ) -> GlobalRenderNodeKey {
        self.inner.insert_node(Box::new(node))
    }
}

impl AnyGlobalRenderNodeTrait for GlobalRenderGraph {
    fn exec(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] encoder: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        render_ctx: RenderContext,
    ) {
        self.iter().for_each(|n| {
            n.exec(encoder, render_ctx);
        });
    }
}

pub trait AnyGlobalRenderNodeTrait {
    fn exec(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        render_ctx: RenderContext,
    );
}
impl<T: GlobalRenderNodeTrait> AnyGlobalRenderNodeTrait for T {
    fn exec(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] encoder: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        render_ctx: RenderContext,
    ) {
        self.run(
            encoder,
            <Self as GlobalRenderNodeTrait>::Query::query(render_ctx.render_packets),
            render_ctx,
        );
    }
}

pub trait GlobalRenderNodeTrait {
    type Query: RenderPacketsQuery;
    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] encoder: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        render_packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        render_ctx: RenderContext,
    );
}

pub trait RenderPacketsQuery {
    type Output<'s>;
    fn query(store: &RenderPackets) -> Self::Output<'_>;
}

/// A marker trait to make compiler happy.
pub trait RenderPacketMark {}
impl RenderPacketMark for VItemRenderInstance {}
impl RenderPacketMark for ViewportGpuPacket {}

impl<T: RenderPacketMark + Send + Sync + 'static> RenderPacketsQuery for T {
    type Output<'s> = &'s [Handle<T>];
    fn query(store: &RenderPackets) -> Self::Output<'_> {
        store.get()
    }
}

impl RenderPacketsQuery for () {
    type Output<'s> = ();
    fn query(_store: &RenderPackets) -> Self::Output<'_> {}
}

macro_rules! impl_tuple_render_packet_query {
    ($($T:ident),*) => {
        impl<$($T: RenderPacketMark + Send + Sync + 'static,)*> RenderPacketsQuery for ($($T,)*) {
            type Output<'s> = ($(&'s [Handle<$T>],)*);
            fn query(store: &RenderPackets) -> Self::Output<'_> {
                ($(store.get::<$T>(),)*)
            }
        }
    };
}

all_tuples!(impl_tuple_render_packet_query, 1, 16, T);

/// A type-erased container of [`Handle`]s for render packets.
///
/// Its inner is a [`TypeBinnedVec`].
#[derive(Default)]
pub struct RenderPackets {
    inner: TypeBinnedVec,
}

impl RenderPackets {
    #[inline]
    pub fn get<T: RenderPacketMark + Send + Sync + 'static>(&self) -> &[Handle<T>] {
        self.inner.get_row::<Handle<T>>()
    }
    #[inline]
    pub fn extend<T: RenderPacketMark + Send + Sync + 'static>(
        &mut self,
        packets: impl IntoIterator<Item = Handle<T>>,
    ) {
        self.inner.extend(packets);
    }
    #[inline]
    pub fn push<T: RenderPacketMark + Send + Sync + 'static>(&mut self, packet: Handle<T>) {
        self.inner.push(packet);
    }
    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}
