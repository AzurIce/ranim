use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use crate::{
    primitives::{Primitive, RenderResource},
    utils::{ReadbackWgpuTexture, WgpuContext},
};

/// A render resource.
pub(crate) trait GpuResource {
    fn new(ctx: &WgpuContext) -> Self
    where
        Self: Sized;
}

/// A storage for pipelines
#[derive(Default)]
pub struct PipelinesPool {
    inner: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl PipelinesPool {
    pub(crate) fn get_or_init<P: GpuResource + Send + Sync + 'static>(
        &mut self,
        ctx: &WgpuContext,
    ) -> &P {
        let id = std::any::TypeId::of::<P>();
        self.inner
            .entry(id)
            .or_insert_with(|| {
                let pipeline = P::new(ctx);
                Box::new(pipeline)
            })
            .downcast_ref::<P>()
            .unwrap()
    }
}

// MARK: RenderTextures
/// Texture resources used for rendering
#[allow(unused)]
pub struct RenderTextures {
    pub render_texture: ReadbackWgpuTexture,
    // multisample_texture: wgpu::Texture,
    pub depth_stencil_texture: ReadbackWgpuTexture,
    pub render_view: wgpu::TextureView,
    pub linear_render_view: wgpu::TextureView,
    // pub(crate) multisample_view: wgpu::TextureView,
    pub(crate) depth_stencil_view: wgpu::TextureView,
}

pub(crate) const OUTPUT_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
impl RenderTextures {
    pub(crate) fn new(ctx: &WgpuContext, width: usize, height: usize) -> Self {
        let format = OUTPUT_TEXTURE_FORMAT;
        let render_texture = ReadbackWgpuTexture::new(
            ctx,
            &wgpu::TextureDescriptor {
                label: Some("Target Texture"),
                size: wgpu::Extent3d {
                    width: width as u32,
                    height: height as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    wgpu::TextureFormat::Rgba8Unorm,
                ],
            },
        );
        // let multisample_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
        //     label: Some("Multisample Texture"),
        //     size: wgpu::Extent3d {
        //         width: width as u32,
        //         height: height as u32,
        //         depth_or_array_layers: 1,
        //     },
        //     mip_level_count: 1,
        //     sample_count: 4,
        //     dimension: wgpu::TextureDimension::D2,
        //     format,
        //     usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        //     view_formats: &[
        //         wgpu::TextureFormat::Rgba8UnormSrgb,
        //         wgpu::TextureFormat::Rgba8Unorm,
        //     ],
        // });
        let depth_stencil_texture = ReadbackWgpuTexture::new(
            ctx,
            &wgpu::TextureDescriptor {
                label: Some("Depth Stencil Texture"),
                size: wgpu::Extent3d {
                    width: width as u32,
                    height: height as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            },
        );
        let render_view = render_texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(format),
            ..Default::default()
        });
        let linear_render_view = render_texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            ..Default::default()
        });
        // let multisample_view = multisample_texture.create_view(&wgpu::TextureViewDescriptor {
        //     format: Some(format),
        //     ..Default::default()
        // });
        let depth_stencil_view =
            depth_stencil_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            render_texture,
            // multisample_texture,
            depth_stencil_texture,
            render_view,
            linear_render_view,
            // multisample_view,
            depth_stencil_view,
        }
    }
}

slotmap::new_key_type! { pub struct RenderInstanceKey; }

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
//
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
