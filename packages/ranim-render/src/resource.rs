use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

use image::{ImageBuffer, Luma, Rgba};

use crate::utils::{ReadbackWgpuTexture, WgpuContext};

/// A render resource.
pub(crate) trait GpuResource {
    fn new(ctx: &WgpuContext) -> Self
    where
        Self: Sized;
}

/// A storage for pipelines
#[derive(bevy_ecs::prelude::Resource, Default)]
pub struct PipelinesPool {
    inner: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl PipelinesPool {
    pub(crate) fn get_or_init<P: GpuResource + Send + Sync + 'static>(
        &self,
        ctx: &WgpuContext,
    ) -> Arc<P> {
        let id = std::any::TypeId::of::<P>();
        {
            let inner = self.inner.read().unwrap();
            if let Some(pipeline) = inner.get(&id) {
                return pipeline.clone().downcast::<P>().unwrap();
            }
        }
        let mut inner = self.inner.write().unwrap();
        inner
            .entry(id)
            .or_insert_with(|| {
                let pipeline = P::new(ctx);
                Arc::new(pipeline)
            })
            .clone()
            .downcast::<P>()
            .unwrap()
    }
}

// MARK: RenderTextures
#[derive(Clone)]
pub(crate) struct RenderTextureState(Arc<RenderTextureStateInner>);

struct RenderTextureStateInner {
    output_dirty: AtomicBool,
    depth_dirty: AtomicBool,
}

impl Default for RenderTextureState {
    fn default() -> Self {
        Self(Arc::new(RenderTextureStateInner {
            output_dirty: AtomicBool::new(true),
            depth_dirty: AtomicBool::new(true),
        }))
    }
}

impl RenderTextureState {
    pub(crate) fn mark_dirty(&self) {
        self.0.output_dirty.store(true, Ordering::Release);
        self.0.depth_dirty.store(true, Ordering::Release);
    }
}

/// Texture resources used for rendering
#[allow(unused)]
pub struct RenderTextures {
    width: u32,
    height: u32,
    pub render_texture: ReadbackWgpuTexture,
    // multisample_texture: wgpu::Texture,
    pub depth_stencil_texture: ReadbackWgpuTexture,
    pub render_view: wgpu::TextureView,
    pub linear_render_view: wgpu::TextureView,
    pub depth_texture_view: wgpu::TextureView,
    /// Bind group for depth texture (used in OIT resolve)
    pub(crate) depth_bind_group: wgpu::BindGroup,
    // pub(crate) multisample_view: wgpu::TextureView,
    pub(crate) depth_stencil_view: wgpu::TextureView,

    state: RenderTextureState,
}

pub(crate) const OUTPUT_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
impl RenderTextures {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    pub(crate) fn new(ctx: &WgpuContext, width: u32, height: u32) -> Self {
        let format = OUTPUT_TEXTURE_FORMAT;
        let render_texture = ReadbackWgpuTexture::new(
            ctx,
            &wgpu::TextureDescriptor {
                label: Some("Target Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
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
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::TEXTURE_BINDING,
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

        let depth_texture_view = depth_stencil_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Depth Texture View"),
            aspect: wgpu::TextureAspect::DepthOnly,
            ..Default::default()
        });

        // Create depth bind group for OIT resolve
        use crate::pipelines::OITResolvePipeline;
        let depth_bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Depth Texture Bind Group"),
            layout: &OITResolvePipeline::depth_bind_group_layout(ctx),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&depth_texture_view),
            }],
        });

        Self {
            width,
            height,
            render_texture,
            // multisample_texture,
            depth_stencil_texture,
            render_view,
            linear_render_view,
            depth_texture_view,
            depth_bind_group,
            // multisample_view,
            depth_stencil_view,
            state: RenderTextureState::default(),
        }
    }

    /// Mark textures as dirty after rendering.
    pub fn mark_dirty(&self) {
        self.state.mark_dirty();
    }

    pub(crate) fn state(&self) -> RenderTextureState {
        self.state.clone()
    }

    /// Start async readback of the output texture (non-blocking).
    pub fn start_readback(&mut self, ctx: &WgpuContext) {
        self.render_texture.start_readback(ctx);
        self.state.0.output_dirty.store(false, Ordering::Release);
    }

    /// Finish a pending async readback, copying data into the CPU-side buffer.
    pub fn finish_readback(&mut self, ctx: &WgpuContext) {
        self.render_texture.finish_readback(ctx);
    }

    /// Try to finish a pending readback without blocking.
    /// Returns `true` if the readback completed (or there was nothing pending),
    /// `false` if the GPU hasn't finished yet.
    pub fn try_finish_readback(&mut self, ctx: &WgpuContext) -> bool {
        self.render_texture.try_finish_readback(ctx)
    }

    pub fn get_rendered_texture_data(&mut self, ctx: &WgpuContext) -> &[u8] {
        if !self.state.0.output_dirty.load(Ordering::Acquire) {
            return self.render_texture.texture_data();
        }
        self.state.0.output_dirty.store(false, Ordering::Release);
        self.render_texture.update_texture_data(ctx)
    }

    pub fn get_rendered_texture_img_buffer(
        &mut self,
        ctx: &WgpuContext,
    ) -> ImageBuffer<Rgba<u8>, &[u8]> {
        ImageBuffer::from_raw(self.width, self.height, self.get_rendered_texture_data(ctx)).unwrap()
    }

    pub fn get_depth_texture_data(&mut self, ctx: &WgpuContext) -> &[f32] {
        if !self.state.0.depth_dirty.load(Ordering::Acquire) {
            return bytemuck::cast_slice(self.depth_stencil_texture.texture_data());
        }
        self.state.0.depth_dirty.store(false, Ordering::Release);
        bytemuck::cast_slice(self.depth_stencil_texture.update_texture_data(ctx))
    }

    pub fn get_depth_texture_img_buffer(
        &mut self,
        ctx: &WgpuContext,
    ) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        let data = self
            .get_depth_texture_data(ctx)
            .iter()
            .map(|&d| (d.clamp(0.0, 1.0) * 255.0) as u8)
            .collect::<Vec<_>>();
        ImageBuffer::from_raw(self.width, self.height, data).unwrap()
    }
}
