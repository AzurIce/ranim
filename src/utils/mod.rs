/// Bezier related stuffs
pub mod bezier;
/// Math stuffs
pub mod math;
/// The rate functions
pub mod rate_functions;
/// Svg related stuffs
pub mod svg;
/// Typst related stuffs
pub mod typst;
pub(crate) mod wgpu;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use glam::{Mat3, Vec2, Vec3, vec2, vec3};

use crate::{render::RenderResource, utils::wgpu::WgpuContext};

// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// pub struct Id(u128);

// impl Default for Id {
//     fn default() -> Self {
//         Self::new()
//     }
// }

// impl Id {
//     pub fn new() -> Self {
//         Self(uuid::Uuid::new_v4().as_u128())
//     }
// }

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

// #[derive(Debug, Clone, Copy)]
// pub enum SubpathWidth {
//     Inner(f32),
//     Outer(f32),
//     Middle(f32),
// }

// impl Default for SubpathWidth {
//     fn default() -> Self {
//         Self::Middle(1.0)
//     }
// }

/// Projects a 3D point onto a plane defined by a unit normal vector.
pub fn project(p: Vec3, unit_normal: Vec3) -> Vec3 {
    // trace!("projecting {:?} by {:?}", p, unit_normal);
    // trace!("dot: {:?}", unit_normal.dot(p));
    // trace!("res: {:?}", p - unit_normal * unit_normal.dot(p));
    p - unit_normal * unit_normal.dot(p)
}

/// Generate basis vecs for a surface from a unit normal vec
pub fn generate_basis(unit_normal: Vec3) -> (Vec3, Vec3) {
    // trace!("generating basis for {:?}", unit_normal);
    let u = if unit_normal.x != 0.0 || unit_normal.y != 0.0 {
        vec3(-unit_normal.y, unit_normal.x, 0.0)
    } else {
        vec3(1.0, 0.0, 0.0)
    }
    .normalize();
    let v = unit_normal.cross(u).normalize();
    (u, v)
}

/// Get a 3d point's 2d coordinate on a 3d plane
pub fn convert_to_2d(p: Vec3, origin: Vec3, basis: (Vec3, Vec3)) -> Vec2 {
    // trace!("converting {:?} by {:?} and {:?}", p, origin, basis);
    let p_local = p - origin;
    vec2(basis.0.dot(p_local), basis.1.dot(p_local))
}

/// Get a 2d point's 3d coordinate on a 3d plane
pub fn convert_to_3d(p: Vec2, origin: Vec3, basis: (Vec3, Vec3)) -> Vec3 {
    origin + basis.0 * p.x + basis.1 * p.y
}

/// Get a rotation matrix from `v1` to `v2`
pub fn rotation_between_vectors(v1: Vec3, v2: Vec3) -> Mat3 {
    // trace!("rotation_between_vectors: v1: {:?}, v2: {:?}", v1, v2);

    if (v2 - v1).length() < f32::EPSILON {
        return Mat3::IDENTITY;
    }
    let mut axis = v1.cross(v2);
    if axis.length() < f32::EPSILON {
        axis = v1.cross(Vec3::Y);
    }
    if axis.length() < f32::EPSILON {
        axis = v1.cross(Vec3::Z);
    }
    // trace!("axis: {:?}", axis);

    let angle = angle_between_vectors(v1, v2);
    // trace!("angle: {:?}", angle);
    Mat3::from_axis_angle(axis, angle)
}

/// Get angle between vectors
pub fn angle_between_vectors(v1: Vec3, v2: Vec3) -> f32 {
    if v1.length() == 0.0 || v2.length() == 0.0 {
        return 0.0;
    }

    (v1.dot(v2) / (v1.length() * v2.length()))
        .clamp(-1.0, 1.0)
        .acos()
}

/// Resize the vec while preserving the order
pub fn resize_preserving_order<T: Clone>(vec: &[T], new_len: usize) -> Vec<T> {
    let indices = (0..new_len).map(|i| i * vec.len() / new_len);
    indices.map(|i| vec[i].clone()).collect()
}

/// Extend the vec with last element
pub fn extend_with_last<T: Clone + Default>(vec: &mut Vec<T>, new_len: usize) {
    let v = vec![vec.last().cloned().unwrap_or_default(); new_len - vec.len()];
    vec.extend(v)
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
