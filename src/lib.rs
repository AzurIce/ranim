use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    ops::Deref,
    sync::Arc,
};

use renderer::RenderResource;
use wgpu::util::DeviceExt;

pub use glam;
pub use bevy_color;

pub mod animation;
pub mod camera;
/// Rabjects are the objects that can be manuplated and rendered
pub mod rabject;
pub(crate) mod renderer;
pub mod scene;
pub mod utils;

#[derive(Default)]
pub struct RenderResourceStorage {
    inner: HashMap<TypeId, Box<dyn Any>>,
}

impl RenderResourceStorage {
    pub fn get_or_init<P: RenderResource + 'static>(&mut self, ctx: &WgpuContext) -> &P {
        let id = std::any::TypeId::of::<P>();
        if !self.inner.contains_key(&id) {
            let pipeline = P::new(ctx);
            self.inner.insert(id, Box::new(pipeline));
        }
        self.inner.get(&id).unwrap().downcast_ref::<P>().unwrap()
    }
    pub fn get_or_init_mut<P: RenderResource + 'static>(&mut self, ctx: &WgpuContext) -> &mut P {
        let id = std::any::TypeId::of::<P>();
        if !self.inner.contains_key(&id) {
            let pipeline = P::new(ctx);
            self.inner.insert(id, Box::new(pipeline));
        }
        self.inner
            .get_mut(&id)
            .unwrap()
            .downcast_mut::<P>()
            .unwrap()
    }
}

pub struct RanimContext {
    pub(crate) wgpu_ctx: Arc<WgpuContext>,
    pub pipelines: RenderResourceStorage,
    pub renderers: RenderResourceStorage,
}

impl RanimContext {
    pub fn new() -> Self {
        let wgpu_ctx = Arc::new(pollster::block_on(WgpuContext::new()));
        let pipelines = RenderResourceStorage::default();
        let renderers = RenderResourceStorage::default();

        Self {
            wgpu_ctx,
            pipelines,
            renderers,
        }
    }

    pub fn wgpu_ctx(&self) -> Arc<WgpuContext> {
        self.wgpu_ctx.clone()
    }
}

pub struct WgpuContext {
    // pub instance: wgpu::Instance,
    // pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl WgpuContext {
    pub async fn new() -> Self {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();

        Self {
            // instance,
            // adapter,
            device,
            queue,
        }
    }
}

pub(crate) struct WgpuBuffer<T: bytemuck::Pod + bytemuck::Zeroable + Debug> {
    pub buffer: wgpu::Buffer,
    usage: wgpu::BufferUsages,
    len: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: bytemuck::Pod + bytemuck::Zeroable + Debug> Deref for WgpuBuffer<T> {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<T: bytemuck::Pod + bytemuck::Zeroable + Debug> WgpuBuffer<T> {
    pub(crate) fn new(ctx: &WgpuContext, size: u64, usage: wgpu::BufferUsages) -> Self {
        Self {
            buffer: ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Simple Vertex Buffer"),
                size,
                usage,
                mapped_at_creation: false,
            }),
            usage,
            len: 0,
            _phantom: std::marker::PhantomData,
        }
    }

    pub(crate) fn new_init(ctx: &WgpuContext, data: &[T], usage: wgpu::BufferUsages) -> Self {
        // trace!("[WgpuBuffer]: new_init, {} {:?}", data.len(), usage);
        Self {
            buffer: ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Simple Vertex Buffer"),
                    contents: bytemuck::cast_slice(data),
                    usage,
                }),
            usage,
            len: data.len(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn prepare_from_slice(&mut self, ctx: &WgpuContext, data: &[T]) {
        if self.size() < std::mem::size_of_val(data) as u64 {
            self.buffer = ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Simple Vertex Buffer"),
                    contents: bytemuck::cast_slice(data),
                    usage: self.usage,
                });
        } else {
            {
                let mut view = ctx
                    .queue
                    .write_buffer_with(
                        self,
                        0,
                        wgpu::BufferSize::new(std::mem::size_of_val(data) as u64).unwrap(),
                    )
                    .unwrap();
                view.copy_from_slice(bytemuck::cast_slice(data));
            }
            ctx.queue.submit([]);
        }
        self.len = data.len();
    }
}
