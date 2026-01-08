/// Primitive for vitem
pub mod vitem;
pub mod vitem2d;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use variadics_please::all_tuples;

use crate::{
    Camera, RenderTextures,
    pipelines::{
        ClipBox2dPipeline, Map3dTo2dPipeline, VItem2dColorPipeline, VItem2dDepthPipeline,
        VItemPipeline,
    },
    primitives::{vitem::VItemRenderInstance, vitem2d::VItem2dRenderInstance},
    utils::{PipelinesStorage, WgpuContext},
};

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

pub trait RenderPacket {}

pub struct VItemComputeRenderNode;

type VItemRenderPacket = VItemRenderInstance;
impl RenderPacket for VItemRenderPacket {}

type VItem2dRenderPacket = VItem2dRenderInstance;
impl RenderPacket for VItem2dRenderPacket {}

pub trait RenderNodeTrait {
    type Query: RenderPacketsQuery;
    fn run(
        &self,
        packets_query_output: <Self::Query as RenderPacketsQuery>::Output<'_>,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        pipelines: &mut PipelinesStorage,
        render_textures: &RenderTextures,
        camera_state: &Camera,
        ctx: &WgpuContext,
    );
    fn exec(
        &self,
        packets_store: &RenderPacketsStore,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        pipelines: &mut PipelinesStorage,
        render_textures: &RenderTextures,
        camera_state: &Camera,
        ctx: &WgpuContext,
    ) {
        self.run(
            Self::Query::query(packets_store),
            #[cfg(not(feature = "profiling"))]
            encoder,
            #[cfg(feature = "profiling")]
            scope,
            pipelines,
            render_textures,
            camera_state,
            ctx,
        );
    }
}

impl RenderNodeTrait for VItemComputeRenderNode {
    type Query = (VItemRenderPacket, VItem2dRenderPacket);

    fn run(
        &self,
        (vitem_packets, vitem2d_packets): <Self::Query as RenderPacketsQuery>::Output<'_>,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        pipelines: &mut PipelinesStorage,
        _render_textures: &RenderTextures,
        camera_state: &Camera,
        ctx: &WgpuContext,
    ) {
        #[cfg(feature = "profiling")]
        let mut scope = scope.scope("Compute Pass");
        // VItem Compute Pass
        {
            #[cfg(feature = "profiling")]
            let mut cpass = scope.scoped_compute_pass("VItem Map Points Compute Pass");
            #[cfg(not(feature = "profiling"))]
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VItem Map Points Compute Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(pipelines.get_or_init::<Map3dTo2dPipeline>(ctx));
            cpass.set_bind_group(0, &camera_state.uniforms_bind_group.bind_group, &[]);

            vitem_packets
                .iter()
                .for_each(|vitem| vitem.encode_compute_pass_command(&mut cpass));
        }
        // VItem2d Compute Pass
        {
            #[cfg(feature = "profiling")]
            let mut cpass = scope.scoped_compute_pass("VItem2d Map Points Compute Pass");
            #[cfg(not(feature = "profiling"))]
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VItem2d Map Points Compute Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(pipelines.get_or_init::<ClipBox2dPipeline>(ctx));

            vitem2d_packets
                .iter()
                .for_each(|vitem| vitem.encode_compute_pass_command(&mut cpass));
        }
    }
}

pub struct VItem2dDepthNode;

impl RenderNodeTrait for VItem2dDepthNode {
    type Query = VItem2dRenderPacket;
    fn run(
        &self,
        vitem2d_packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        pipelines: &mut PipelinesStorage,
        render_textures: &RenderTextures,
        camera_state: &Camera,
        ctx: &WgpuContext,
    ) {
        #[cfg(feature = "profiling")]
        let mut scope = scope.scope("Depth Render Pass");
        // VItem2d Depth Render Pass
        {
            let RenderTextures {
                depth_stencil_view, ..
            } = &render_textures;
            let rpass_desc = wgpu::RenderPassDescriptor {
                label: Some("VItem2d Depth Render Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_stencil_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            };
            #[cfg(feature = "profiling")]
            let mut rpass = scope.scoped_render_pass("VItem2d Depth Render Pass", rpass_desc);
            #[cfg(not(feature = "profiling"))]
            let mut rpass = encoder.begin_render_pass(&rpass_desc);
            rpass.set_pipeline(pipelines.get_or_init::<VItem2dDepthPipeline>(ctx));
            rpass.set_bind_group(0, &camera_state.uniforms_bind_group.bind_group, &[]);
            vitem2d_packets
                .iter()
                .for_each(|vitem| vitem.encode_depth_render_pass_command(&mut rpass));
        }
    }
}

pub struct VItemRenderNode;

impl RenderNodeTrait for VItemRenderNode {
    type Query = VItemRenderPacket;
    fn run(
        &self,
        vitem_packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        pipelines: &mut PipelinesStorage,
        render_textures: &RenderTextures,
        camera_state: &Camera,
        ctx: &WgpuContext,
    ) {
        let RenderTextures {
            // multisample_view,
            render_view,
            depth_stencil_view,
            ..
        } = render_textures;
        let rpass_desc = wgpu::RenderPassDescriptor {
            label: Some("VItem Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                // view: multisample_view,
                // resolve_target: Some(render_view),
                depth_slice: None,
                view: render_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_stencil_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        };
        #[cfg(feature = "profiling")]
        let mut rpass = scope.scoped_render_pass("VItem Render Pass", rpass_desc);
        #[cfg(not(feature = "profiling"))]
        let mut rpass = encoder.begin_render_pass(&rpass_desc);
        rpass.set_pipeline(pipelines.get_or_init::<VItemPipeline>(ctx));
        rpass.set_bind_group(0, &camera_state.uniforms_bind_group.bind_group, &[]);
        vitem_packets
            .iter()
            .for_each(|vitem| vitem.encode_render_pass_command(&mut rpass));
    }
}

pub struct VItem2dRenderNode;

impl RenderNodeTrait for VItem2dRenderNode {
    type Query = VItem2dRenderPacket;
    fn run(
        &self,
        vitem2d_packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        pipelines: &mut PipelinesStorage,
        render_textures: &RenderTextures,
        camera_state: &Camera,
        ctx: &WgpuContext,
    ) {
        // VItem2d Render Pass
        let RenderTextures {
            render_view,
            depth_stencil_view,
            ..
        } = render_textures;
        let rpass_desc = wgpu::RenderPassDescriptor {
            label: Some("VItem2d Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_stencil_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        };
        #[cfg(feature = "profiling")]
        let mut rpass = scope.scoped_render_pass("VItem2d Render Pass", rpass_desc);
        #[cfg(not(feature = "profiling"))]
        let mut rpass = encoder.begin_render_pass(&rpass_desc);
        rpass.set_pipeline(pipelines.get_or_init::<VItem2dColorPipeline>(ctx));
        rpass.set_bind_group(0, &camera_state.uniforms_bind_group.bind_group, &[]);
        vitem2d_packets
            .iter()
            .for_each(|vitem| vitem.encode_render_pass_command(&mut rpass));
    }
}

pub trait RenderPacketsQuery {
    type Output<'s>;
    fn query(store: &RenderPacketsStore) -> Self::Output<'_>;
}

impl<T: RenderPacket + 'static> RenderPacketsQuery for T {
    type Output<'s> = &'s [T];
    fn query(store: &RenderPacketsStore) -> Self::Output<'_> {
        store.get_packets()
    }
}

macro_rules! impl_tuple_render_packet_query {
    ($($T:ident),*) => {
        impl<$($T: RenderPacket + 'static,)*> RenderPacketsQuery for ($($T,)*) {
            type Output<'s> = ($(&'s [$T],)*);
            fn query(store: &RenderPacketsStore) -> Self::Output<'_> {
                ($(store.get_packets::<$T>(),)*)
            }
        }
    };
}

all_tuples!(impl_tuple_render_packet_query, 1, 16, T);

/// A trait to support calling `clear` on the type erased trait object.
pub trait AnyRenderPackets: Any {
    fn clear(&mut self);
}

impl<T: Any> AnyRenderPackets for Vec<T> {
    fn clear(&mut self) {
        self.clear();
    }
}

#[derive(Default)]
pub struct RenderPacketsStore {
    pub packets: HashMap<TypeId, Box<dyn AnyRenderPackets>>,
}

impl RenderPacketsStore {
    pub fn init_packets<T: RenderPacket + 'static>(&mut self) -> &mut Vec<T> {
        let entry = self
            .packets
            .entry(TypeId::of::<T>())
            .or_insert(Box::<Vec<T>>::default());
        (entry.as_mut() as &mut dyn Any)
            .downcast_mut::<Vec<T>>()
            .unwrap()
    }
    pub fn get_packets<T: RenderPacket + 'static>(&self) -> &[T] {
        self.packets
            .get(&TypeId::of::<T>())
            .and_then(|v| (v.as_ref() as &dyn Any).downcast_ref::<Vec<T>>())
            .map(|v| v.as_ref())
            .unwrap_or(&[])
    }
    pub fn extend<T: RenderPacket + 'static>(&mut self, packets: impl IntoIterator<Item = T>) {
        (self.init_packets::<T>() as &mut dyn Any)
            .downcast_mut::<Vec<T>>()
            .unwrap()
            .extend(packets);
    }
    pub fn push<T: RenderPacket + 'static>(&mut self, packet: T) {
        (self.init_packets::<T>() as &mut dyn Any)
            .downcast_mut::<Vec<T>>()
            .unwrap()
            .push(packet);
    }
    pub fn clear(&mut self) {
        self.packets.iter_mut().for_each(|(_, v)| {
            v.clear();
        });
    }
}

/// The RenderCommand encodes the commands.
#[deprecated]
pub trait RenderCommand {
    /// Encode the compute pass command
    fn encode_compute_pass_command(&self, cpass: &mut wgpu::ComputePass);
    fn encode_depth_render_pass_command(&self, rpass: &mut wgpu::RenderPass);
    /// Encode the render pass command
    fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass);

    /// Debug
    fn debug(&self, _ctx: &WgpuContext) {}
}

macro_rules! impl_tuple_render_command {
    ($($T:ident),*) => {
        impl<$($T: RenderCommand,)*> RenderCommand for ($($T,)*) {
            fn encode_compute_pass_command(
                &self,
                cpass: &mut wgpu::ComputePass,
            ) {
                #[allow(non_snake_case, reason = "`all_tuples!()` generates non-snake-case variable names.")]
                let ($($T,)*) = self;
                $($T.encode_compute_pass_command(
                    cpass,
                );)*
            }
            fn encode_depth_render_pass_command(
                &self,
                rpass: &mut wgpu::RenderPass,
            ) {
                #[allow(non_snake_case, reason = "`all_tuples!()` generates non-snake-case variable names.")]
                let ($($T,)*) = self;
                $($T.encode_depth_render_pass_command(
                    rpass,
                );)*
            }
            fn encode_render_pass_command(
                &self,
                rpass: &mut wgpu::RenderPass,
            ) {
                #[allow(non_snake_case, reason = "`all_tuples!()` generates non-snake-case variable names.")]
                let ($($T,)*) = self;
                $($T.encode_render_pass_command(
                    rpass,
                );)*
            }
            fn debug(&self, ctx: &WgpuContext) {
                #[allow(non_snake_case, reason = "`all_tuples!()` generates non-snake-case variable names.")]
                let ($($T,)*) = self;
                $($T.debug(ctx);)*
            }
        }
    };
}

all_tuples!(impl_tuple_render_command, 1, 16, T);

impl<T: RenderCommand> RenderCommand for Vec<T> {
    fn encode_compute_pass_command(&self, cpass: &mut wgpu::ComputePass) {
        self.iter()
            .for_each(|x| x.encode_compute_pass_command(cpass))
    }
    fn encode_depth_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        self.iter()
            .for_each(|x| x.encode_depth_render_pass_command(rpass))
    }
    fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        self.iter()
            .for_each(|x| x.encode_render_pass_command(rpass))
    }
    fn debug(&self, _ctx: &WgpuContext) {
        self.iter().for_each(|x| x.debug(_ctx));
    }
}

impl<T: RenderCommand, const N: usize> RenderCommand for [T; N] {
    fn encode_compute_pass_command(&self, cpass: &mut wgpu::ComputePass) {
        self.iter()
            .for_each(|x| x.encode_compute_pass_command(cpass))
    }
    fn encode_depth_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        self.iter()
            .for_each(|x| x.encode_depth_render_pass_command(rpass))
    }
    fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        self.iter()
            .for_each(|x| x.encode_render_pass_command(rpass))
    }
    fn debug(&self, _ctx: &WgpuContext) {
        self.iter().for_each(|x| x.debug(_ctx));
    }
}

/// The Primitive is the basic renderable object in Ranim.
///
/// The Primitive itself is simply the data of the object.
/// A Primitive has a corresponding [`Primitive::RenderInstance`],
/// which implements [`RenderResource`] and [`RenderCommand`]:
/// - [`RenderResource`]: A trait about init or update itself with [`RenderResource::Data`].
/// - [`RenderCommand`]: A trait about encoding GPU commands.
pub trait Primitive {
    /// The RenderInstance
    type RenderInstance: RenderResource<Data = Self> + RenderCommand + Send + Sync + 'static;
}

slotmap::new_key_type! { pub struct RenderInstanceKey; }

// MARK: RenderPool
#[derive(Default)]
pub struct RenderPool {
    #[allow(clippy::type_complexity)]
    inner: slotmap::SlotMap<
        RenderInstanceKey,
        (Arc<RenderInstanceKey>, TypeId, Box<dyn AnyRenderCommand>),
    >,
    last_frame_dropped: HashMap<TypeId, Vec<RenderInstanceKey>>,
}

impl RenderPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: RenderInstanceKey) -> Option<&dyn AnyRenderCommand> {
        self.inner
            .get(key)
            .map(|x| x.2.as_ref() as &dyn AnyRenderCommand)
    }

    pub fn show(&self) {
        self.inner
            .iter()
            .enumerate()
            .for_each(|(idx, (_, (k, _, _)))| {
                print!("{idx}: {}, ", Arc::strong_count(k));
            });
        println!();
    }

    pub fn alloc<P: Primitive>(&mut self, ctx: &WgpuContext, data: &P) -> Arc<RenderInstanceKey> {
        let last_frame_dropped = self
            .last_frame_dropped
            .entry(TypeId::of::<P::RenderInstance>())
            .or_default();
        if let Some(key) = last_frame_dropped.pop() {
            let entry = self.inner.get_mut(key).unwrap();
            let key = entry.0.clone();
            (entry.2.as_mut() as &mut dyn Any)
                .downcast_mut::<P::RenderInstance>()
                .unwrap()
                .update(ctx, data);
            key
        } else {
            let handle = self.inner.insert_with_key(|key| {
                (
                    Arc::new(key),
                    TypeId::of::<P::RenderInstance>(),
                    Box::new(P::RenderInstance::init(ctx, data)),
                )
            });
            self.inner.get(handle).unwrap().0.clone()
        }
    }

    pub fn clean(&mut self) {
        self.inner.retain(|key, (_, t_id, _)| {
            self.last_frame_dropped
                .get(t_id)
                .map(|x| !x.contains(&key))
                .unwrap_or(true)
        });
        // println!("dropped {}", self.last_frame_dropped.len());
        self.last_frame_dropped.clear();
        self.inner
            .iter()
            .filter(|(_, (key, _, _))| Arc::strong_count(key) == 1)
            .for_each(|(key, (_, t_id, _))| {
                self.last_frame_dropped.entry(*t_id).or_default().push(key);
            });
    }
}

/// Type erased [`RenderCommand`]
pub trait AnyRenderCommand: Any + RenderCommand + Send + Sync {}
impl<T: RenderCommand + Any + Send + Sync> AnyRenderCommand for T {}

impl RenderCommand for Vec<&dyn RenderCommand> {
    fn encode_compute_pass_command(&self, cpass: &mut wgpu::ComputePass) {
        for render_instance in self {
            render_instance.encode_compute_pass_command(cpass);
        }
    }
    fn encode_depth_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        for render_instance in self {
            render_instance.encode_depth_render_pass_command(rpass);
        }
    }
    fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        for render_instance in self {
            render_instance.encode_render_pass_command(rpass);
        }
    }
    fn debug(&self, ctx: &WgpuContext) {
        for render_instance in self {
            render_instance.debug(ctx);
        }
    }
}
