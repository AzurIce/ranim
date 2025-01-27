use std::{fmt::Debug, ops::Deref};

use log::trace;
use wgpu::util::DeviceExt;

use crate::context::WgpuContext;

pub(crate) struct WgpuBuffer<T: bytemuck::Pod + bytemuck::Zeroable + Debug> {
    label: Option<&'static str>,
    buffer: wgpu::Buffer,
    usage: wgpu::BufferUsages,
    inner: Option<T>,
}

impl<T: bytemuck::Pod + bytemuck::Zeroable + Debug> Deref for WgpuBuffer<T> {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<T: bytemuck::Pod + bytemuck::Zeroable + Debug> WgpuBuffer<T> {
    pub(crate) fn new(
        ctx: &WgpuContext,
        label: Option<&'static str>,
        usage: wgpu::BufferUsages,
        size: u64,
    ) -> Self {
        assert!(
            usage.contains(wgpu::BufferUsages::COPY_DST),
            "Buffer {label:?} does not contains COPY_DST"
        );
        Self {
            label,
            buffer: ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label,
                size,
                usage,
                mapped_at_creation: false,
            }),
            usage,
            inner: None,
        }
    }

    pub(crate) fn new_init(
        ctx: &WgpuContext,
        label: Option<&'static str>,
        usage: wgpu::BufferUsages,
        data: T,
    ) -> Self {
        assert!(
            usage.contains(wgpu::BufferUsages::COPY_DST),
            "Buffer {label:?} does not contains COPY_DST"
        );
        // trace!("[WgpuBuffer]: new_init, {} {:?}", data.len(), usage);
        Self {
            label,
            buffer: ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label,
                    contents: bytemuck::bytes_of(&data),
                    usage,
                }),
            usage,
            inner: Some(data),
        }
    }

    pub(crate) fn get(&self) -> Option<&T> {
        self.inner.as_ref()
    }

    pub(crate) fn set(&mut self, ctx: &WgpuContext, data: T) {
        if self.buffer.size() < std::mem::size_of_val::<T>(&data) as u64 {
            self.buffer = ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: self.label,
                    contents: bytemuck::bytes_of(&data),
                    usage: self.usage,
                });
        } else {
            {
                let mut view = ctx
                    .queue
                    .write_buffer_with(
                        self,
                        0,
                        wgpu::BufferSize::new(std::mem::size_of_val(&data) as u64).unwrap(),
                    )
                    .unwrap();
                view.copy_from_slice(bytemuck::bytes_of(&data));
            }
            ctx.queue.submit([]);
        }
    }
}

pub(crate) struct WgpuVecBuffer<T: Default + bytemuck::Pod + bytemuck::Zeroable + Debug> {
    label: Option<&'static str>,
    buffer: wgpu::Buffer,
    usage: wgpu::BufferUsages,
    /// Keep match to the buffer size
    inner: Vec<T>,
}

impl<T: Default + bytemuck::Pod + bytemuck::Zeroable + Debug> Deref for WgpuVecBuffer<T> {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<T: Default + bytemuck::Pod + bytemuck::Zeroable + Debug> WgpuVecBuffer<T> {
    pub(crate) fn new(
        ctx: &WgpuContext,
        label: Option<&'static str>,
        usage: wgpu::BufferUsages,
        len: usize,
    ) -> Self {
        assert!(
            usage.contains(wgpu::BufferUsages::COPY_DST),
            "Buffer {label:?} does not contains COPY_DST"
        );
        Self {
            label,
            buffer: ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label,
                size: (std::mem::size_of::<T>() * len) as u64,
                usage,
                mapped_at_creation: false,
            }),
            usage,
            inner: vec![T::default(); len],
        }
    }

    pub(crate) fn new_init(
        ctx: &WgpuContext,
        label: Option<&'static str>,
        usage: wgpu::BufferUsages,
        data: &[T],
    ) -> Self {
        assert!(
            usage.contains(wgpu::BufferUsages::COPY_DST),
            "Buffer {label:?} does not contains COPY_DST"
        );
        // trace!("[WgpuBuffer]: new_init, {} {:?}", data.len(), usage);
        Self {
            label,
            buffer: ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label,
                    contents: bytemuck::cast_slice(&data),
                    usage,
                }),
            usage,
            inner: data.to_vec(),
        }
    }

    pub(crate) fn get(&self) -> &[T] {
        self.inner.as_ref()
    }

    pub(crate) fn resize(&mut self, ctx: &WgpuContext, len: usize) -> bool {
        let size = (std::mem::size_of::<T>() * len) as u64;
        let resize = self.buffer.size() != size;
        if resize {
            self.inner.resize(len, T::default());
            self.buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: self.label,
                size,
                usage: self.usage,
                mapped_at_creation: false,
            })
        }
        resize
    }

    pub(crate) fn set(&mut self, ctx: &WgpuContext, data: &[T]) {
        // trace!("{} {}", self.inner.len(), data.len());
        self.inner.resize(data.len(), T::default());
        self.inner.copy_from_slice(data);

        if self.buffer.size() != (std::mem::size_of::<T>() * data.len()) as u64 {
            self.buffer = ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: self.label,
                    contents: bytemuck::cast_slice(&data),
                    usage: self.usage,
                });
        } else {
            {
                let mut view = ctx
                    .queue
                    .write_buffer_with(
                        self,
                        0,
                        wgpu::BufferSize::new((std::mem::size_of::<T>() * data.len()) as u64)
                            .unwrap(),
                    )
                    .unwrap();
                view.copy_from_slice(bytemuck::cast_slice(&data));
            }
            ctx.queue.submit([]);
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test() {
        // let x = vec![0, 1, 2, 3];
        // assert_eq!(
        //     bytemuck::bytes_of(&[x.as_slice()]),
        //     bytemuck::bytes_of(&x)
        // )
    }
}
