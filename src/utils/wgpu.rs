use std::{fmt::Debug, ops::Deref};

use wgpu::util::DeviceExt;

use crate::context::WgpuContext;

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

    pub fn set_len(&mut self, len: usize) {
        self.len = len;
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
