use ranim::{
    pipeline::simple::{SimplePipeline, SimpleVertex},
    WgpuBuffer, WgpuContext,
};

const TEXTURE_DIMS: (usize, usize) = (512, 512);

async fn run() {
    let mut texture_data = vec![0u8; TEXTURE_DIMS.0 * TEXTURE_DIMS.1 * 4];

    let ctx = WgpuContext::new().await;

    let render_target = ctx.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: TEXTURE_DIMS.0 as u32,
            height: TEXTURE_DIMS.1 as u32,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
    });
    let output_staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: texture_data.capacity() as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let pipeline = SimplePipeline::new(&ctx);

    let data = SimpleVertex::test_data();
    // context setup done

    let vertex_buffer = WgpuBuffer::new_init(&ctx, &data, wgpu::BufferUsages::VERTEX);
    let texture_view = render_target.create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

    {
        pipeline.render(&mut encoder, &texture_view, &vertex_buffer);
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &render_target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &output_staging_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some((TEXTURE_DIMS.0 * 4) as u32),
                    rows_per_image: Some(TEXTURE_DIMS.1 as u32),
                },
            },
            render_target.size(),
        );
        ctx.queue.submit(Some(encoder.finish()));
    }

    {
        let buffer_slice = output_staging_buffer.slice(..);

        // NOTE: We have to create the mapping THEN device.poll() before await
        // the future. Otherwise the application will freeze.
        let (tx, rx) = async_channel::bounded(1);
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send_blocking(result).unwrap()
        });
        ctx.device.poll(wgpu::Maintain::Wait).panic_on_timeout();
        rx.recv().await.unwrap().unwrap();

        {
            let view = buffer_slice.get_mapped_range();
            texture_data.copy_from_slice(&view);
        }

        use image::{ImageBuffer, Rgba};
        let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(
            TEXTURE_DIMS.0 as u32,
            TEXTURE_DIMS.1 as u32,
            texture_data,
        )
        .unwrap();
        buffer.save("image.png").unwrap();
    }
    output_staging_buffer.unmap();
}
fn main() {
    println!("Hello, world!");
    pollster::block_on(run())
}
