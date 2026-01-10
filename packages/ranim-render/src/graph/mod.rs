pub mod vitem_color;
pub use vitem_color::*;
pub mod vitem_compute;
pub use vitem_compute::*;
pub mod vitem_depth;
pub use vitem_depth::*;

use slotmap::{SecondaryMap, SlotMap};
use variadics_please::all_tuples;

use crate::{
    RenderContext, ViewportGpuPacket,
    primitives::{vitem::VItemRenderInstance, vitem2d::VItem2dRenderInstance},
    resource::Handle,
    utils::collections::TypeBinnedVec,
};

slotmap::new_key_type! { pub struct RenderNodeKey; }

#[derive(Default)]
pub struct RenderGraph {
    nodes: SlotMap<RenderNodeKey, Box<dyn AnyRenderNodeTrait + Send + Sync>>,
    nexts: SecondaryMap<RenderNodeKey, Vec<RenderNodeKey>>,
    prevs: SecondaryMap<RenderNodeKey, Vec<RenderNodeKey>>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert_node(
        &mut self,
        node: impl AnyRenderNodeTrait + Send + Sync + 'static,
    ) -> RenderNodeKey {
        let key = self.nodes.insert(Box::new(node));
        self.nexts.insert(key, Vec::new());
        self.prevs.insert(key, Vec::new());
        key
    }
    pub fn insert_edge(&mut self, from: RenderNodeKey, to: RenderNodeKey) {
        self.nexts.get_mut(from).unwrap().push(to);
        self.prevs.get_mut(to).unwrap().push(from);
    }
    pub fn iter(&self) -> RenderGraphTopoIter<'_> {
        RenderGraphTopoIter::new(self)
    }
}

pub struct RenderGraphTopoIter<'a> {
    graph: &'a RenderGraph,
    in_degrees: SecondaryMap<RenderNodeKey, usize>,
    ready_stack: Vec<RenderNodeKey>,
}

impl<'a> RenderGraphTopoIter<'a> {
    pub fn new(graph: &'a RenderGraph) -> Self {
        let mut in_degrees = SecondaryMap::new();
        let mut ready_stack = Vec::new();

        for (key, _) in graph.nodes.iter() {
            let degree = graph.prevs[key].len();
            in_degrees.insert(key, degree);

            if degree == 0 {
                ready_stack.push(key);
            }
        }

        Self {
            graph,
            in_degrees,
            ready_stack,
        }
    }
}

impl<'a> Iterator for RenderGraphTopoIter<'a> {
    type Item = &'a Box<dyn AnyRenderNodeTrait + Send + Sync>;

    fn next(&mut self) -> Option<Self::Item> {
        let current_key = self.ready_stack.pop()?;

        let next_nodes = self.graph.nexts.get(current_key).unwrap();
        for &next_key in next_nodes {
            let degree = self.in_degrees.get_mut(next_key).unwrap();
            *degree -= 1;
            if *degree == 0 {
                self.ready_stack.push(next_key);
            }
        }

        self.graph.nodes.get(current_key)
    }
}

impl AnyRenderNodeTrait for RenderGraph {
    fn exec(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        render_packets: &RenderPackets,
        render_ctx: &mut RenderContext,
        viewport: &ViewportGpuPacket,
    ) {
        self.iter().for_each(|n| {
            n.exec(
                #[cfg(not(feature = "profiling"))]
                encoder,
                #[cfg(feature = "profiling")]
                scope,
                render_packets,
                render_ctx,
                viewport,
            );
        });
    }
}

pub trait AnyRenderNodeTrait {
    fn exec(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        render_packets: &RenderPackets,
        render_ctx: &mut RenderContext,
        viewport: &ViewportGpuPacket,
    );
}

impl<T: RenderNodeTrait> AnyRenderNodeTrait for T {
    fn exec(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        render_packets: &RenderPackets,
        render_ctx: &mut RenderContext,
        viewport: &ViewportGpuPacket,
    ) {
        self.run(
            #[cfg(not(feature = "profiling"))]
            encoder,
            #[cfg(feature = "profiling")]
            scope,
            <Self as RenderNodeTrait>::Query::query(render_packets),
            render_ctx,
            viewport,
        );
    }
}

pub trait RenderNodeTrait {
    type Query: RenderPacketsQuery;
    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        render_packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        render_ctx: &mut RenderContext,
        viewport: &ViewportGpuPacket,
    );
}

pub trait RenderPacketsQuery {
    type Output<'s>;
    fn query(store: &RenderPackets) -> Self::Output<'_>;
}

/// A marker trait to make compiler happy.
pub trait RenderPacketMark {}
impl RenderPacketMark for VItemRenderInstance {}
impl RenderPacketMark for VItem2dRenderInstance {}

impl<T: RenderPacketMark + Send + Sync + 'static> RenderPacketsQuery for T {
    type Output<'s> = &'s [Handle<T>];
    fn query(store: &RenderPackets) -> Self::Output<'_> {
        store.get()
    }
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
