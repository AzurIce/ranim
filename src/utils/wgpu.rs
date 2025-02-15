use std::fmt::Debug;

use wgpu::util::DeviceExt;

use crate::context::WgpuContext;

#[allow(unused)]
pub(crate) struct WgpuBuffer<T: bytemuck::Pod + bytemuck::Zeroable + Debug> {
    label: Option<&'static str>,
    buffer: wgpu::Buffer,
    usage: wgpu::BufferUsages,
    inner: T,
}

impl<T: bytemuck::Pod + bytemuck::Zeroable + Debug> AsRef<wgpu::Buffer> for WgpuBuffer<T> {
    fn as_ref(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

#[allow(unused)]
impl<T: bytemuck::Pod + bytemuck::Zeroable + Debug> WgpuBuffer<T> {
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
            inner: data,
        }
    }

    pub(crate) fn get(&self) -> &T {
        &self.inner
    }

    pub(crate) fn set(&mut self, ctx: &WgpuContext, data: T) {
        {
            let mut view = ctx
                .queue
                .write_buffer_with(
                    &self.buffer,
                    0,
                    wgpu::BufferSize::new(std::mem::size_of_val(&data) as u64).unwrap(),
                )
                .unwrap();
            view.copy_from_slice(bytemuck::bytes_of(&data));
        }
        ctx.queue.submit([]);
        self.inner = data;
    }
}

pub(crate) struct WgpuVecBuffer<T: Default + bytemuck::Pod + bytemuck::Zeroable + Debug> {
    label: Option<&'static str>,
    pub(crate) buffer: Option<wgpu::Buffer>,
    usage: wgpu::BufferUsages,
    /// Keep match to the buffer size
    inner: Vec<T>,
}

impl<T: Default + bytemuck::Pod + bytemuck::Zeroable + Debug> WgpuVecBuffer<T> {
    pub(crate) fn new(label: Option<&'static str>, usage: wgpu::BufferUsages) -> Self {
        assert!(
            usage.contains(wgpu::BufferUsages::COPY_DST),
            "Buffer {label:?} does not contains COPY_DST"
        );
        Self {
            label,
            buffer: None,
            usage,
            inner: vec![],
        }
    }

    pub(crate) fn get(&self) -> &[T] {
        self.inner.as_ref()
    }

    pub(crate) fn resize(&mut self, ctx: &WgpuContext, len: usize) -> bool {
        let size = (std::mem::size_of::<T>() * len) as u64;
        let realloc = self
            .buffer
            .as_ref()
            .map(|b| b.size() != size)
            .unwrap_or(true);
        if realloc {
            self.inner.resize(len, T::default());
            self.buffer = Some(ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: self.label,
                size,
                usage: self.usage,
                mapped_at_creation: false,
            }))
        }
        realloc
    }

    pub(crate) fn set(&mut self, ctx: &WgpuContext, data: &[T]) -> bool {
        // trace!("{} {}", self.inner.len(), data.len());
        self.inner.resize(data.len(), T::default());
        self.inner.copy_from_slice(data);
        let realloc = self
            .buffer
            .as_ref()
            .map(|b| b.size() != (std::mem::size_of_val(data)) as u64)
            .unwrap_or(true);

        if realloc {
            self.buffer = Some(
                ctx.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: self.label,
                        contents: bytemuck::cast_slice(data),
                        usage: self.usage,
                    }),
            );
        } else {
            {
                let mut view = ctx
                    .queue
                    .write_buffer_with(
                        self.buffer.as_ref().unwrap(),
                        0,
                        wgpu::BufferSize::new((std::mem::size_of_val(data)) as u64).unwrap(),
                    )
                    .unwrap();
                view.copy_from_slice(bytemuck::cast_slice(data));
            }
            ctx.queue.submit([]);
        }
        realloc
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
