/// Primitive for vitem
pub mod vitem;
pub mod vitem2d;

use crate::{
    graph::{RenderNodeTrait, RenderPacketsQuery},
    utils::RenderContext,
};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

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

pub struct VItemComputeRenderNode;

impl RenderNodeTrait for VItemComputeRenderNode {
    type Query = (VItemRenderInstance, VItem2dRenderInstance);

    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        (vitem_packets, vitem2d_packets): <Self::Query as RenderPacketsQuery>::Output<'_>,
        ctx: &mut RenderContext,
        camera_state: &Camera,
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
            cpass.set_pipeline(ctx.pipelines.get_or_init::<Map3dTo2dPipeline>(ctx.wgpu_ctx));
            cpass.set_bind_group(0, &camera_state.uniforms_bind_group.bind_group, &[]);

            vitem_packets
                .iter()
                .map(|h| ctx.render_pool.get_packet(h))
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
            cpass.set_pipeline(ctx.pipelines.get_or_init::<ClipBox2dPipeline>(ctx.wgpu_ctx));

            vitem2d_packets
                .iter()
                .map(|h| ctx.render_pool.get_packet(h))
                .for_each(|vitem| vitem.encode_compute_pass_command(&mut cpass));
        }
    }
}

pub struct VItem2dDepthNode;

impl RenderNodeTrait for VItem2dDepthNode {
    type Query = VItem2dRenderInstance;
    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        vitem2d_packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        ctx: &mut RenderContext,
        camera_state: &Camera,
    ) {
        #[cfg(feature = "profiling")]
        let mut scope = scope.scope("Depth Render Pass");
        // VItem2d Depth Render Pass
        {
            let RenderTextures {
                depth_stencil_view, ..
            } = ctx.render_textures;
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
            rpass.set_pipeline(
                ctx.pipelines
                    .get_or_init::<VItem2dDepthPipeline>(ctx.wgpu_ctx),
            );
            rpass.set_bind_group(0, &camera_state.uniforms_bind_group.bind_group, &[]);
            vitem2d_packets
                .iter()
                .map(|h| ctx.render_pool.get_packet(h))
                .for_each(|vitem| vitem.encode_depth_render_pass_command(&mut rpass));
        }
    }
}

pub struct VItemRenderNode;

impl RenderNodeTrait for VItemRenderNode {
    type Query = VItemRenderInstance;
    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        vitem_packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        ctx: &mut RenderContext,
        camera_state: &Camera,
    ) {
        let RenderTextures {
            // multisample_view,
            render_view,
            depth_stencil_view,
            ..
        } = ctx.render_textures;
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
        rpass.set_pipeline(ctx.pipelines.get_or_init::<VItemPipeline>(ctx.wgpu_ctx));
        rpass.set_bind_group(0, &camera_state.uniforms_bind_group.bind_group, &[]);
        vitem_packets
            .iter()
            .map(|h| ctx.render_pool.get_packet(h))
            .for_each(|vitem| vitem.encode_render_pass_command(&mut rpass));
    }
}

pub struct VItem2dRenderNode;

impl RenderNodeTrait for VItem2dRenderNode {
    type Query = VItem2dRenderInstance;
    fn run(
        &self,
        #[cfg(not(feature = "profiling"))] encoder: &mut wgpu::CommandEncoder,
        #[cfg(feature = "profiling")] scope: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        vitem2d_packets: <Self::Query as RenderPacketsQuery>::Output<'_>,
        ctx: &mut RenderContext,
        camera_state: &Camera,
    ) {
        // VItem2d Render Pass
        let RenderTextures {
            render_view,
            depth_stencil_view,
            ..
        } = ctx.render_textures;
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
        rpass.set_pipeline(
            ctx.pipelines
                .get_or_init::<VItem2dColorPipeline>(ctx.wgpu_ctx),
        );
        rpass.set_bind_group(0, &camera_state.uniforms_bind_group.bind_group, &[]);
        vitem2d_packets
            .iter()
            .map(|h| ctx.render_pool.get_packet(h))
            .for_each(|vitem| vitem.encode_render_pass_command(&mut rpass));
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
    type RenderPacket: RenderResource<Data = Self> + Send + Sync + 'static;
}

slotmap::new_key_type! { pub struct RenderInstanceKey; }

// MARK: RenderPool
#[derive(Default)]
pub struct RenderPool {
    #[allow(clippy::type_complexity)]
    inner: slotmap::SlotMap<
        RenderInstanceKey,
        (
            Arc<RenderInstanceKey>,
            TypeId,
            Box<dyn Any + Send + Sync + 'static>,
        ),
    >,
    last_frame_dropped: HashMap<TypeId, Vec<RenderInstanceKey>>,
}

/// A handle to a render packet.
///
/// In its inner is an [`Arc`] reference count of the [`RenderInstanceKey`].
pub struct Handle<T> {
    key: Arc<RenderInstanceKey>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl RenderPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_packet<T: 'static>(&self, handle: &Handle<T>) -> &T {
        self.get(*handle.key)
            .map(|x| x.downcast_ref::<T>().unwrap())
            .unwrap()
    }

    pub fn alloc_packet<P: Primitive>(
        &mut self,
        ctx: &WgpuContext,
        data: &P,
    ) -> Handle<P::RenderPacket> {
        let key = self.alloc(ctx, data);
        Handle {
            key,
            _phantom: std::marker::PhantomData,
        }
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

    fn get(&self, key: RenderInstanceKey) -> Option<&(dyn Any + Send + Sync + 'static)> {
        self.inner.get(key).map(|x| x.2.as_ref())
    }

    fn alloc<P: Primitive>(&mut self, ctx: &WgpuContext, data: &P) -> Arc<RenderInstanceKey> {
        let last_frame_dropped = self
            .last_frame_dropped
            .entry(TypeId::of::<P::RenderPacket>())
            .or_default();
        if let Some(key) = last_frame_dropped.pop() {
            let entry = self.inner.get_mut(key).unwrap();
            let key = entry.0.clone();
            (entry.2.as_mut() as &mut dyn Any)
                .downcast_mut::<P::RenderPacket>()
                .unwrap()
                .update(ctx, data);
            key
        } else {
            let handle = self.inner.insert_with_key(|key| {
                (
                    Arc::new(key),
                    TypeId::of::<P::RenderPacket>(),
                    Box::new(P::RenderPacket::init(ctx, data)),
                )
            });
            self.inner.get(handle).unwrap().0.clone()
        }
    }

    /// When called, all instances not referenced are recorded into the `last_frame_dropped` map.
    /// An will be cleaned in the next call.
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
