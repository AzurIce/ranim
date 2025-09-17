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
    iter::Sum,
    ops::Div,
};
#[cfg(not(target_arch = "wasm32"))]
use std::{
    env::current_exe,
    io::{BufReader, Read},
    path::{Path, PathBuf},
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

/// Get data's avg
pub fn avg<T: Clone + Sum + Div<f64, Output = T>>(data: &[T]) -> T {
    data.iter().cloned().sum::<T>() / data.len() as f64
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

/// Resize the vec while preserving the order
///
/// returns the repeated idxs
/// ```
///                     *     *     *     *  repeated
/// [0, 1, 2, 3] -> [0, 0, 1, 1, 2, 2, 3 ,3]
/// ```
pub fn resize_preserving_order_with_repeated_indices<T: Clone>(
    vec: &[T],
    new_len: usize,
) -> (Vec<T>, Vec<usize>) {
    let mut res = Vec::with_capacity(new_len);
    let mut added_idxs = Vec::with_capacity(new_len);
    let mut prev_index = None;
    for i in 0..new_len {
        let index = i * vec.len() / new_len;
        if prev_index.map(|i| i == index).unwrap_or(false) {
            added_idxs.push(res.len());
        }
        res.push(vec[index].clone());
        prev_index = Some(index);
    }
    (res, added_idxs)
}

/// Resize the vec while preserving the order
///
/// returns the repeated cnt of each value
/// ```
///                 [2  2][2  2][2  2][2  2]
/// [0, 1, 2, 3] -> [0, 0, 1, 1, 2, 2, 3 ,3]
/// ```
pub fn resize_preserving_order_with_repeated_cnt<T: Clone>(
    vec: &[T],
    new_len: usize,
) -> (Vec<T>, Vec<usize>) {
    let mut res = Vec::with_capacity(new_len);
    let mut cnts = vec![0; vec.len()];

    let mut src_indices = Vec::with_capacity(new_len);
    for i in 0..new_len {
        let index = i * vec.len() / new_len;
        cnts[index] += 1;
        res.push(vec[index].clone());
        src_indices.push(index);
    }
    (res, src_indices.into_iter().map(|i| cnts[i]).collect())
}

/// Extend the vec with last element
pub fn extend_with_last<T: Clone + Default>(vec: &mut Vec<T>, new_len: usize) {
    let v = vec![vec.last().cloned().unwrap_or_default(); new_len - vec.len()];
    vec.extend(v)
}

// f.a + b.a * (1.0 - f.a)
fn merge_alpha(alpha: f32, n: usize) -> f32 {
    let mut result = alpha;
    for _ in 1..n {
        result = result + (1.0 - result) * alpha;
    }
    result
}

/// Get a target alpha value that can get value of given alpha after mixed n times
pub fn apart_alpha(alpha: f32, n: usize, eps: f32) -> f32 {
    if alpha == 0.0 {
        return 0.0;
    }
    let mut left = (0.0, 0.0);
    let mut right = (1.0, 1.0);

    while right.0 - left.0 > eps {
        let mid_single = (left.0 + right.0) / 2.0;
        let mid_merged = merge_alpha(mid_single, n);

        if (mid_merged - alpha).abs() < f32::EPSILON {
            return mid_single;
        }

        if mid_merged < alpha {
            left = (mid_single, mid_merged);
        } else {
            right = (mid_single, mid_merged);
        }
    }

    ((left.0 + right.0) / 2.0).clamp(0.0, 1.0)
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

const FFMPEG_RELEASE_URL: &str = "https://github.com/eugeneware/ffmpeg-static/releases/latest";

#[allow(unused)]
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn exe_dir() -> PathBuf {
    current_exe().unwrap().parent().unwrap().to_path_buf()
}

/// Download latest release of ffmpeg from <https://github.com/eugeneware/ffmpeg-static/releases/latest> to <target_dir>/ffmpeg
#[cfg(not(target_arch = "wasm32"))]
pub fn download_ffmpeg(target_dir: impl AsRef<Path>) -> Result<PathBuf, anyhow::Error> {
    use anyhow::Context;
    use std::io::Cursor;

    use itertools::Itertools;
    use log::info;

    let target_dir = target_dir.as_ref();

    let res = reqwest::blocking::get(FFMPEG_RELEASE_URL).context("failed to get release url")?;
    let url = res.url().to_string();
    let url = url.split("tag").collect_array::<2>().unwrap();
    let url = format!("{}/download/{}", url[0], url[1]);
    info!("ffmpeg release url: {url:?}");

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    let url = format!("{url}/ffmpeg-win32-x64.gz");
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let url = format!("{url}/ffmpeg-linux-x64.gz");
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    let url = format!("{url}/ffmpeg-linux-arm64.gz");
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let url = format!("{url}/ffmpeg-darwin-x64.gz");
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let url = format!("{url}/ffmpeg-darwin-arm64.gz");

    info!("downloading ffmpeg from {url:?}...");

    let res = reqwest::blocking::get(&url).context("get err")?;
    let mut decoder =
        flate2::bufread::GzDecoder::new(BufReader::new(Cursor::new(res.bytes().unwrap())));
    let mut bytes = Vec::new();
    decoder
        .read_to_end(&mut bytes)
        .context("GzDecoder decode err")?;
    let ffmpeg_path = target_dir.join("ffmpeg");
    std::fs::write(&ffmpeg_path, bytes).unwrap();

    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::set_permissions(&ffmpeg_path, std::fs::Permissions::from_mode(0o755))?;
    }
    info!("ffmpeg downloaded to {target_dir:?}");
    Ok(ffmpeg_path)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_resize_preserve_order_with_repeated_cnt() {
        let values = vec![0, 1, 2, 3];
        let (v, c) = resize_preserving_order_with_repeated_cnt(&values, 8);
        assert_eq!(v, vec![0, 0, 1, 1, 2, 2, 3, 3]);
        assert_eq!(c, vec![2; 8]);
    }

    #[test]
    fn tset_apart_alpha() {
        let a = apart_alpha(1.0, 10, 1e-3);
        println!("{a}");
        println!("{}", merge_alpha(1.0, 10));
    }
}
