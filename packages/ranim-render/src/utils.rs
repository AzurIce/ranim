use std::{fmt::Debug, marker::PhantomData, ops::Deref};

use bytemuck::AnyBitPattern;
use tracing::{info, warn};
use wgpu::util::DeviceExt;

pub mod collections {
    use std::{
        any::{Any, TypeId},
        collections::HashMap,
    };

    /// A trait to support calling `clear` on the type erased trait object.
    pub trait AnyClear: Any + Send + Sync {
        fn clear(&mut self);
    }

    impl<T: Any + Send + Sync> AnyClear for Vec<T> {
        fn clear(&mut self) {
            self.clear();
        }
    }

    /// A type-erased container for render packets.
    ///
    /// Basically a HashMap of `TypeId` -> type-erased `Vec<T>`
    #[derive(Default)]
    pub struct TypeBinnedVec {
        inner: HashMap<TypeId, Box<dyn AnyClear>>,
    }

    impl TypeBinnedVec {
        fn init_row<T: Send + Sync + 'static>(&mut self) -> &mut Vec<T> {
            #[allow(clippy::unwrap_or_default)]
            let entry = self
                .inner
                .entry(TypeId::of::<T>())
                .or_insert(Box::<Vec<T>>::default());
            (entry.as_mut() as &mut dyn Any)
                .downcast_mut::<Vec<T>>()
                .unwrap()
        }
        pub fn get_row<T: Send + Sync + 'static>(&self) -> &[T] {
            self.inner
                .get(&TypeId::of::<T>())
                .and_then(|v| (v.as_ref() as &dyn Any).downcast_ref::<Vec<T>>())
                .map(|v| v.as_ref())
                .unwrap_or(&[])
        }
        pub fn extend<T: Send + Sync + 'static>(&mut self, packets: impl IntoIterator<Item = T>) {
            self.init_row::<T>().extend(packets);
        }
        pub fn push<T: Send + Sync + 'static>(&mut self, packet: T) {
            self.init_row::<T>().push(packet);
        }
        pub fn clear(&mut self) {
            self.inner.iter_mut().for_each(|(_, v)| {
                v.clear();
            });
        }
    }
}

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
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
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
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        pollster::block_on(rx.recv()).unwrap().unwrap();

        buffer_slice.get_mapped_range().to_vec()
    }
}

pub(crate) struct WgpuVecBuffer<T: Default + bytemuck::Pod + bytemuck::Zeroable + Debug> {
    label: Option<&'static str>,
    pub(crate) buffer: wgpu::Buffer,
    usage: wgpu::BufferUsages,
    /// Keep match to the buffer size
    len: usize,
    _phantom: PhantomData<T>,
    // inner: Vec<T>,
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
        let size = (std::mem::size_of::<T>() * len) as u64;
        Self {
            label,
            buffer: ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: label,
                size,
                usage: usage,
                mapped_at_creation: false,
            }),
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
        let mut buffer = Self::new(ctx, label, usage, data.len());
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
        let realloc = self.buffer.size() != size;
        if realloc {
            self.len = len;
            // self.inner.resize(len, T::default());
            self.buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: self.label,
                size,
                usage: self.usage,
                mapped_at_creation: false,
            })
        }
        realloc
    }

    pub(crate) fn set(&mut self, ctx: &WgpuContext, data: &[T]) -> bool {
        // trace!("{} {}", self.inner.len(), data.len());
        // self.inner.resize(data.len(), T::default());
        // self.inner.copy_from_slice(data);
        self.len = data.len();
        let realloc = self.buffer.size() != std::mem::size_of_val(data) as u64;

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
            self.buffer = buffer;
        } else {
            // info!("queue copy");
            {
                let mut view = ctx
                    .queue
                    .write_buffer_with(
                        &self.buffer,
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

        encoder.copy_buffer_to_buffer(&self.buffer, 0, &staging_buffer, 0, size as u64);
        ctx.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = async_channel::bounded(1);
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.try_send(result).unwrap()
        });
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        pollster::block_on(rx.recv()).unwrap().unwrap();

        let x = buffer_slice.get_mapped_range().to_vec();
        Some(x)
    }
}

pub struct WgpuTexture {
    inner: wgpu::Texture,
}

impl WgpuTexture {
    pub fn new(ctx: &WgpuContext, desc: &wgpu::TextureDescriptor) -> Self {
        Self {
            inner: ctx.device.create_texture(desc),
        }
    }
}

impl Deref for WgpuTexture {
    type Target = wgpu::Texture;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// A [`WgpuTexture`] with [`wgpu::TextureUsages::COPY_SRC`] usage and wrapped with a staging buffer and
/// a cpu side bytes `Vec<T>` buffer to read back from the texture.
pub struct ReadbackWgpuTexture {
    inner: WgpuTexture,
    aligned_bytes_per_row: usize,
    staging_buffer: wgpu::Buffer,
    bytes: Vec<u8>,
}

impl Deref for ReadbackWgpuTexture {
    type Target = WgpuTexture;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

const ALIGNMENT: usize = 256;
impl ReadbackWgpuTexture {
    pub fn new(ctx: &WgpuContext, desc: &wgpu::TextureDescriptor) -> Self {
        if !desc.usage.contains(wgpu::TextureUsages::COPY_SRC) {
            warn!(
                "ReadbackWgpuTexture should have COPY_SRC usage, but got {:?}, will auto add this usage",
                desc.usage
            );
        }
        let texture = WgpuTexture::new(
            ctx,
            &wgpu::TextureDescriptor {
                usage: desc.usage | wgpu::TextureUsages::COPY_SRC,
                ..*desc
            },
        );
        let block_size = desc.format.block_copy_size(None).unwrap();
        let bytes_per_row =
            (texture.size().width * block_size).div_ceil(ALIGNMENT as u32) as usize * ALIGNMENT;

        let staging_buffer_label = desc.label.map(|s| format!("{s} Staging Buffer"));
        let staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: staging_buffer_label.as_deref(),
            size: (bytes_per_row * texture.size().height as usize) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let len =
            (texture.size().width as usize * texture.size().height as usize * block_size as usize);
        let bytes = vec![0u8; len];

        Self {
            inner: texture,
            aligned_bytes_per_row: bytes_per_row,
            staging_buffer,
            bytes,
        }
    }
    pub fn texture_data(&self) -> &[u8] {
        &self.bytes
    }
    pub fn update_texture_data(&mut self, ctx: &WgpuContext) -> &[u8] {
        let size = self.size();

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Get Texture Data"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: self,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.aligned_bytes_per_row as u32),
                    rows_per_image: Some(size.height),
                },
            },
            size,
        );
        // println!("copying");
        ctx.queue.submit(Some(encoder.finish()));
        pollster::block_on(async {
            let buffer_slice = self.staging_buffer.slice(..);

            // NOTE: We have to create the mapping THEN device.poll() before await
            // the future. Otherwise the application will freeze.
            let (tx, rx) = async_channel::bounded(1);
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.try_send(result);
            });
            // println!("mapping");
            ctx.device
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();
            // println!("mapped");
            rx.recv().await.unwrap().unwrap();

            {
                let view = buffer_slice.get_mapped_range();
                let block_size = self.inner.format().block_copy_size(None).unwrap();
                let bytes_in_row = (size.width * block_size) as usize;
                // dbg!(bytes_in_row, block_size, size);

                // texture_data.copy_from_slice(&view);
                for y in 0..size.height as usize {
                    let src_row_start = y * self.aligned_bytes_per_row;
                    let dst_row_start = y * bytes_in_row;

                    self.bytes[dst_row_start..dst_row_start + bytes_in_row]
                        .copy_from_slice(&view[src_row_start..src_row_start + bytes_in_row]);
                }
            }
        });
        self.staging_buffer.unmap();

        &self.bytes
    }
}

// // Should not be called frequently
// /// Get texture data from a wgpu texture
// #[allow(unused)]
// pub(crate) fn get_texture_data(ctx: &WgpuContext, texture: &::wgpu::Texture) -> Vec<u8> {
//     const ALIGNMENT: usize = 256;
//     use ::wgpu;
//     let bytes_per_row =
//         ((texture.size().width * 4) as f32 / ALIGNMENT as f32).ceil() as usize * ALIGNMENT;
//     let mut texture_data = vec![0u8; bytes_per_row * texture.size().height as usize];

//     let output_staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
//         label: Some("Output Staging Buffer"),
//         size: (bytes_per_row * texture.size().height as usize) as u64,
//         usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
//         mapped_at_creation: false,
//     });

//     let mut encoder = ctx
//         .device
//         .create_command_encoder(&wgpu::CommandEncoderDescriptor {
//             label: Some("Get Texture Data"),
//         });
//     encoder.copy_texture_to_buffer(
//         wgpu::TexelCopyTextureInfo {
//             aspect: wgpu::TextureAspect::All,
//             texture,
//             mip_level: 0,
//             origin: wgpu::Origin3d::ZERO,
//         },
//         wgpu::TexelCopyBufferInfo {
//             buffer: &output_staging_buffer,
//             layout: wgpu::TexelCopyBufferLayout {
//                 offset: 0,
//                 bytes_per_row: Some(bytes_per_row as u32),
//                 rows_per_image: Some(texture.size().height),
//             },
//         },
//         texture.size(),
//     );
//     ctx.queue.submit(Some(encoder.finish()));
//     pollster::block_on(async {
//         let buffer_slice = output_staging_buffer.slice(..);

//         // NOTE: We have to create the mapping THEN device.poll() before await
//         // the future. Otherwise the application will freeze.
//         let (tx, rx) = async_channel::bounded(1);
//         buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
//             pollster::block_on(tx.send(result)).unwrap()
//         });
//         ctx.device
//             .poll(wgpu::PollType::wait_indefinitely())
//             .unwrap();
//         rx.recv().await.unwrap().unwrap();

//         {
//             let view = buffer_slice.get_mapped_range();
//             // texture_data.copy_from_slice(&view);
//             for y in 0..texture.size().height as usize {
//                 let src_row_start = y * bytes_per_row;
//                 let dst_row_start = y * texture.size().width as usize * 4;

//                 texture_data[dst_row_start..dst_row_start + texture.size().width as usize * 4]
//                     .copy_from_slice(
//                         &view[src_row_start..src_row_start + texture.size().width as usize * 4],
//                     );
//             }
//         }
//     });
//     output_staging_buffer.unmap();
//     texture_data
// }

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
