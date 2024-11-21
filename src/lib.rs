use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ops::Deref,
};

use pipeline::RenderPipeline;
use wgpu::util::DeviceExt;

pub use glam;
pub use palette;

pub(crate) mod renderer;
pub(crate) mod pipeline;
pub mod camera;
pub mod animation;
pub mod scene;
pub mod mobject;
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
        self.pipelines.get(&id).unwrap().downcast_ref::<P>().unwrap()
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
                    usage: wgpu::BufferUsages::VERTEX,
                });
        } else {
            ctx.queue.write_buffer(self, 0, bytemuck::cast_slice(data));
        }
    }
}

// /// Sum two matrices.
// #[pyfunction]
// fn sum_matrix(a: Vec<Vec<f64>>, b: Vec<Vec<f64>>) -> Vec<Vec<f64>> {
//     a.into_iter()
//         .zip(b.into_iter())
//         .map(|(a, b)| {
//             a.into_iter()
//                 .zip(b.into_iter())
//                 .map(|(a, b)| a + b)
//                 .collect()
//         })
//         .collect()
// }

// /// A Python module implemented in Rust.
// #[pymodule]
// fn ranim(m: &Bound<'_, PyModule>) -> PyResult<()> {
//     m.add_function(wrap_pyfunction!(sum_matrix, m)?)?;
//     Ok(())
// }

/// Stores custom, user-provided types.
#[derive(Default, Debug)]
pub struct Storage {
    pipelines: HashMap<TypeId, Box<dyn Any + Send>>,
}

impl Storage {
    /// Returns `true` if `Storage` contains a type `T`.
    pub fn has<T: 'static>(&self) -> bool {
        self.pipelines.contains_key(&TypeId::of::<T>())
    }

    /// Inserts the data `T` in to [`Storage`].
    pub fn store<T: 'static + Send>(&mut self, data: T) {
        let _ = self.pipelines.insert(TypeId::of::<T>(), Box::new(data));
    }

    /// Returns a reference to the data with type `T` if it exists in [`Storage`].
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.pipelines.get(&TypeId::of::<T>()).map(|pipeline| {
            pipeline
                .downcast_ref::<T>()
                .expect("Value with this type does not exist in Storage.")
        })
    }

    /// Returns a mutable reference to the data with type `T` if it exists in [`Storage`].
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.pipelines.get_mut(&TypeId::of::<T>()).map(|pipeline| {
            pipeline
                .downcast_mut::<T>()
                .expect("Value with this type does not exist in Storage.")
        })
    }
}
