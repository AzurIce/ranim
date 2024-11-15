use std::ops::Deref;

use pipeline::Pipeline;
// use pyo3::prelude::*;
use wgpu::util::DeviceExt;

pub mod mobject;
pub mod pipeline;

pub trait Renderable {
    type Pipeline: Pipeline;
    fn vertex_data(&self) -> Vec<<Self::Pipeline as Pipeline>::Vertex>;
}

pub struct WgpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,

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
            instance,
            adapter,
            device,
            queue,
        }
    }
}

pub fn foo() {
    println!("Hello, world!");
}

pub struct WgpuBuffer<T: bytemuck::Pod + bytemuck::Zeroable> {
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
    pub fn new(ctx: &WgpuContext, size: u64, usage: wgpu::BufferUsages) -> Self {
        Self {
            buffer: ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Simple Vertex Buffer"),
                size,
                usage,
                mapped_at_creation: false,
            }),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn new_init(ctx: &WgpuContext, data: &[T], usage: wgpu::BufferUsages) -> Self {
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

    pub fn prepare(&mut self, ctx: &WgpuContext, data: &[T]) {
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
