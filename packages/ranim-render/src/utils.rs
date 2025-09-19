use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    marker::PhantomData,
};

use log::info;
use wgpu::util::DeviceExt;

use crate::RenderResource;

/// Wgpu context
pub struct WgpuContext {
    /// The wgpu instance   
    pub instance: wgpu::Instance,
    /// The wgpu adapter
    pub adapter: wgpu::Adapter,
    /// The wgpu device
    pub device: wgpu::Device,
    /// The wgpu queue
    pub queue: wgpu::Queue,
}

impl WgpuContext {
    /// Create a new wgpu context
    pub async fn new() -> Self {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .unwrap();
        info!("wgpu adapter info: {:?}", adapter.get_info());

        #[cfg(feature = "profiling")]
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: adapter.features()
                    & wgpu_profiler::GpuProfiler::ALL_WGPU_TIMER_FEATURES,
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();
        #[cfg(not(feature = "profiling"))]
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
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
        // ctx.queue.submit([]);
        self.inner = data;
    }

    #[allow(unused)]
    pub(crate) fn read_buffer(&self, ctx: &WgpuContext) -> Vec<u8> {
        let size = std::mem::size_of::<T>();
        let staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Debug Staging Buffer"),
            size: size as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Debug Read Encoder"),
            });

        encoder.copy_buffer_to_buffer(&self.buffer, 0, &staging_buffer, 0, size as u64);
        ctx.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = async_channel::bounded(1);
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            pollster::block_on(tx.send(result)).unwrap()
        });
        ctx.device.poll(wgpu::PollType::Wait).unwrap();
        pollster::block_on(rx.recv()).unwrap().unwrap();

        buffer_slice.get_mapped_range().to_vec()
    }
}

pub(crate) struct WgpuVecBuffer<T: Default + bytemuck::Pod + bytemuck::Zeroable + Debug> {
    label: Option<&'static str>,
    pub(crate) buffer: Option<wgpu::Buffer>,
    usage: wgpu::BufferUsages,
    /// Keep match to the buffer size
    len: usize,
    _phantom: PhantomData<T>,
    // inner: Vec<T>,
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
            len: 0,
            _phantom: PhantomData,
            // inner: vec![],
        }
    }

    pub(crate) fn new_init(
        ctx: &WgpuContext,
        label: Option<&'static str>,
        usage: wgpu::BufferUsages,
        data: &[T],
    ) -> Self {
        let mut buffer = Self::new(label, usage);
        buffer.set(ctx, data);
        buffer
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }
    // pub(crate) fn get(&self) -> &[T] {
    //     self.inner.as_ref()
    // }

    pub(crate) fn resize(&mut self, ctx: &WgpuContext, len: usize) -> bool {
        let size = (std::mem::size_of::<T>() * len) as u64;
        let realloc = self
            .buffer
            .as_ref()
            .map(|b| b.size() != size)
            .unwrap_or(true);
        if realloc {
            self.len = len;
            // self.inner.resize(len, T::default());
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
        // self.inner.resize(data.len(), T::default());
        // self.inner.copy_from_slice(data);
        self.len = data.len();
        let realloc = self
            .buffer
            .as_ref()
            .map(|b| b.size() != (std::mem::size_of_val(data)) as u64)
            .unwrap_or(true);

        if realloc {
            // info!("realloc");
            // NOTE: create_buffer_init sometimes causes freezing in wasm
            let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: self.label,
                size: (std::mem::size_of_val(data)) as u64,
                usage: self.usage,
                mapped_at_creation: false,
            });
            ctx.queue
                .write_buffer(&buffer, 0, bytemuck::cast_slice(data));
            // info!("new");
            self.buffer = Some(buffer);
        } else {
            // info!("queue copy");
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
            // ctx.queue.submit([]);
        }
        // info!("done");
        realloc
    }

    #[allow(unused)]
    pub(crate) fn read_buffer(&self, ctx: &WgpuContext) -> Option<Vec<u8>> {
        let buffer = self.buffer.as_ref()?;
        let size = std::mem::size_of::<T>() * self.len;
        let staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Debug Staging Buffer"),
            size: size as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Debug Read Encoder"),
            });

        encoder.copy_buffer_to_buffer(buffer, 0, &staging_buffer, 0, size as u64);
        ctx.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = async_channel::bounded(1);
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            pollster::block_on(tx.send(result)).unwrap()
        });
        ctx.device.poll(wgpu::PollType::Wait).unwrap();
        pollster::block_on(rx.recv()).unwrap().unwrap();

        let x = buffer_slice.get_mapped_range().to_vec();
        Some(x)
    }
}

/// A storage for pipelines
#[derive(Default)]
pub struct PipelinesStorage {
    inner: HashMap<TypeId, Box<dyn Any>>,
}

impl PipelinesStorage {
    pub(crate) fn get_or_init<P: RenderResource + 'static>(&mut self, ctx: &WgpuContext) -> &P {
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
    // pub(crate) fn get_or_init_mut<P: RenderResource + 'static>(
    //     &mut self,
    //     ctx: &WgpuContext,
    // ) -> &mut P {
    //     let id = std::any::TypeId::of::<P>();
    //     self.inner
    //         .entry(id)
    //         .or_insert_with(|| {
    //             let pipeline = P::new(ctx);
    //             Box::new(pipeline)
    //         })
    //         .downcast_mut::<P>()
    //         .unwrap()
    // }
}

// Should not be called frequently
/// Get texture data from a wgpu texture
#[allow(unused)]
pub(crate) fn get_texture_data(ctx: &WgpuContext, texture: &::wgpu::Texture) -> Vec<u8> {
    const ALIGNMENT: usize = 256;
    use ::wgpu;
    let bytes_per_row =
        ((texture.size().width * 4) as f32 / ALIGNMENT as f32).ceil() as usize * ALIGNMENT;
    let mut texture_data = vec![0u8; bytes_per_row * texture.size().height as usize];

    let output_staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Staging Buffer"),
        size: (bytes_per_row * texture.size().height as usize) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Get Texture Data"),
        });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            aspect: wgpu::TextureAspect::All,
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &output_staging_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row as u32),
                rows_per_image: Some(texture.size().height),
            },
        },
        texture.size(),
    );
    ctx.queue.submit(Some(encoder.finish()));
    pollster::block_on(async {
        let buffer_slice = output_staging_buffer.slice(..);

        // NOTE: We have to create the mapping THEN device.poll() before await
        // the future. Otherwise the application will freeze.
        let (tx, rx) = async_channel::bounded(1);
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            pollster::block_on(tx.send(result)).unwrap()
        });
        ctx.device.poll(wgpu::PollType::Wait).unwrap();
        rx.recv().await.unwrap().unwrap();

        {
            let view = buffer_slice.get_mapped_range();
            // texture_data.copy_from_slice(&view);
            for y in 0..texture.size().height as usize {
                let src_row_start = y * bytes_per_row;
                let dst_row_start = y * texture.size().width as usize * 4;

                texture_data[dst_row_start..dst_row_start + texture.size().width as usize * 4]
                    .copy_from_slice(
                        &view[src_row_start..src_row_start + texture.size().width as usize * 4],
                    );
            }
        }
    });
    output_staging_buffer.unmap();
    texture_data
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
