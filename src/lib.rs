use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ops::Deref,
};

use pipeline::RenderPipeline;
use wgpu::util::DeviceExt;

pub use glam;
pub use palette;

/// Blueprints are the data structures that are used to create [`Rabject`]s
pub mod blueprint;
/// Rabjects are the objects that can be manuplated and rendered
pub mod rabject;
pub mod animation;
pub mod camera;
pub mod mobject;
pub(crate) mod pipeline;
/// Renderers implements a whole set of rendering steps
pub(crate) mod renderer;
pub mod scene;
pub mod utils;

pub struct RanimContext {
    pub(crate) wgpu_ctx: WgpuContext,
    pub pipelines: HashMap<TypeId, Box<dyn Any>>,
    // pub renderers: HashMap<TypeId, Box<dyn Any>>,
}

impl RanimContext {
    pub fn new() -> Self {
        let wgpu_ctx = pollster::block_on(WgpuContext::new());
        let pipelines = HashMap::<TypeId, Box<dyn Any>>::new();

        Self {
            wgpu_ctx,
            pipelines,
        }
    }

    pub fn get_or_init_pipeline<P: RenderPipeline + 'static>(&mut self) -> &P {
        let id = std::any::TypeId::of::<P>();
        if !self.pipelines.contains_key(&id) {
            let pipeline = P::new(&self);
            self.pipelines.insert(id, Box::new(pipeline));
        }
        self.pipelines
            .get(&id)
            .unwrap()
            .downcast_ref::<P>()
            .unwrap()
    }
}

pub(crate) struct WgpuContext {
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

pub fn foo() {
    println!("Hello, world!");
}

pub(crate) struct WgpuBuffer<T: bytemuck::Pod + bytemuck::Zeroable> {
    pub buffer: wgpu::Buffer,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: bytemuck::Pod + bytemuck::Zeroable> Deref for WgpuBuffer<T> {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<T: bytemuck::Pod + bytemuck::Zeroable> WgpuBuffer<T> {
    // pub(crate) fn new(ctx: &WgpuContext, size: u64, usage: wgpu::BufferUsages) -> Self {
    //     Self {
    //         buffer: ctx.device.create_buffer(&wgpu::BufferDescriptor {
    //             label: Some("Simple Vertex Buffer"),
    //             size,
    //             usage,
    //             mapped_at_creation: false,
    //         }),
    //         _phantom: std::marker::PhantomData,
    //     }
    // }

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
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn len(&self) -> u64 {
        self.size() / std::mem::size_of::<T>() as u64
    }

    pub(crate) fn prepare_from_slice(&mut self, ctx: &WgpuContext, data: &[T]) {
        if self.size() < std::mem::size_of_val(data) as u64 {
            self.buffer = ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Simple Vertex Buffer"),
                    contents: bytemuck::cast_slice(data),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
        } else {
            ctx.queue.write_buffer(self, 0, bytemuck::cast_slice(data));
        }
    }
}
