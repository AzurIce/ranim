pub mod vitem_color;
use std::ops::{Deref, DerefMut};

pub use vitem_color::*;
pub mod vitem_compute;
pub use vitem_compute::*;
pub mod vitem_depth;
pub use vitem_depth::*;
pub mod oit_resolve;
pub use oit_resolve::*;

use crate::{
    RenderContext,
    graph::{GlobalRenderNodeTrait, RenderPacketsQuery},
    primitives::viewport::ViewportGpuPacket,
    utils::collections::Graph,
};

slotmap::new_key_type! { pub struct ViewRenderNodeKey; }
/// Render graph per-view.
#[derive(Default)]
pub struct ViewRenderGraph {
    inner: Graph<ViewRenderNodeKey, Box<dyn AnyViewRenderNodeTrait + Send + Sync>>,
}

impl Deref for ViewRenderGraph {
    type Target = Graph<ViewRenderNodeKey, Box<dyn AnyViewRenderNodeTrait + Send + Sync>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ViewRenderGraph {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl ViewRenderGraph {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert_node(
        &mut self,
        node: impl AnyViewRenderNodeTrait + Send + Sync + 'static,
    ) -> ViewRenderNodeKey {
        self.inner.insert_node(Box::new(node))
    }
}

impl AnyViewRenderNodeTrait for ViewRenderGraph {
    fn exec(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        render_ctx: RenderContext,
        viewport: &ViewportGpuPacket,
    ) {
        self.iter().for_each(|n| {
            n.exec(
                #[cfg(not(feature = "profiling"))]
                encoder,
                #[cfg(feature = "profiling")]
                scope,
                render_ctx,
                viewport,
            );
        });
    }
}

/// A Render Node that is executed per view.
///
/// The main difference between this and [`super::AnyGlobalRenderNodeTrait`] is that
/// is accepts an extra referenced [`ViewportGpuPacket`].
pub trait ViewRenderNodeTrait {
    type Query: RenderPacketsQuery;
    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        render_packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        render_ctx: RenderContext,
        viewport: &ViewportGpuPacket,
    );
}

pub trait AnyViewRenderNodeTrait {
    fn exec(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        render_ctx: RenderContext,
        viewport: &ViewportGpuPacket,
    );
}

impl<T: ViewRenderNodeTrait> AnyViewRenderNodeTrait for T {
    fn exec(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        render_ctx: RenderContext,
        viewport: &ViewportGpuPacket,
    ) {
        self.run(
            #[cfg(not(feature = "profiling"))]
            encoder,
            #[cfg(feature = "profiling")]
            scope,
            <Self as ViewRenderNodeTrait>::Query::query(render_ctx.render_packets),
            render_ctx,
            viewport,
        );
    }
}

impl GlobalRenderNodeTrait for ViewRenderGraph {
    type Query = ViewportGpuPacket;
    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] encoder: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        viewports: <Self::Query as super::RenderPacketsQuery>::Output<'_>,
        render_ctx: RenderContext,
    ) {
        #[cfg(feature = "profiling")]
        profiling::scope!("render_view");
        for viewport in viewports
            .iter()
            .map(|v| render_ctx.render_pool.get_packet(v))
        {
            self.exec(encoder, render_ctx, viewport);
        }
    }
}
