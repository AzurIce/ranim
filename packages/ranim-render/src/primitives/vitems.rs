use crate::utils::{WgpuContext, WgpuVecBuffer};
use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};

use crate::scene::VItemRenderData;

/// Per-item metadata stored in a GPU buffer.
/// Tells shaders where each VItem's data lives in the merged buffers.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct ItemInfo {
    /// Offset into the merged points buffer
    pub point_offset: u32,
    /// Number of points for this item
    pub point_count: u32,
    /// Offset into the merged attribute buffers (fill_rgbas, stroke_rgbas, stroke_widths)
    pub attr_offset: u32,
    /// Number of attributes (= point_count.div_ceil(2))
    pub attr_count: u32,
}

/// Per-item plane data (normal + origin), stored as array of structs.
/// The origin is the first point of the item (used by vertex shader).
/// basis_u/basis_v are generated deterministically from the normal in the shader.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct PlaneData {
    pub normal: Vec4, // xyz = normal, w = pad
    pub origin: Vec4, // xyz = first point, w = pad
}

/// Merged GPU buffers for all VItems in a frame.
///
/// Instead of one set of buffers per VItem, all data is packed into
/// contiguous arrays with an index table (`item_infos`) that tells
/// shaders where each item's data lives.
pub struct VItemsBuffer {
    /// Per-item metadata: offsets and counts
    pub(crate) item_infos_buffer: WgpuVecBuffer<ItemInfo>,
    /// Per-item plane data (normal + origin for vertex shader)
    pub(crate) planes_buffer: WgpuVecBuffer<PlaneData>,
    /// Per-item clip boxes (5 i32 each: min_x, max_x, min_y, max_y, max_w)
    pub(crate) clip_boxes_buffer: WgpuVecBuffer<i32>,

    /// Merged 3D points from all VItems
    pub(crate) points3d_buffer: WgpuVecBuffer<Vec4>,
    /// Merged 2D projected points (written by compute shader)
    pub(crate) points2d_buffer: WgpuVecBuffer<Vec4>,
    /// Merged fill colors
    pub(crate) fill_rgbas_buffer: WgpuVecBuffer<Vec4>,
    /// Merged stroke colors
    pub(crate) stroke_rgbas_buffer: WgpuVecBuffer<Vec4>,
    /// Merged stroke widths
    pub(crate) stroke_widths_buffer: WgpuVecBuffer<f32>,

    /// Number of items
    pub(crate) item_count: u32,
    /// Total number of points across all items
    pub(crate) total_points: u32,

    /// Compute bind group (recreated when buffers resize)
    pub(crate) compute_bind_group: Option<wgpu::BindGroup>,
    /// Render bind group (recreated when buffers resize)
    pub(crate) render_bind_group: Option<wgpu::BindGroup>,
}

impl VItemsBuffer {
    pub fn new(ctx: &WgpuContext) -> Self {
        // Start with empty buffers (minimum size 1 to avoid zero-size buffer)
        let storage_rw = wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC;
        let storage_ro = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;

        Self {
            item_infos_buffer: WgpuVecBuffer::new(ctx, Some("Merged ItemInfos"), storage_ro, 1),
            planes_buffer: WgpuVecBuffer::new(ctx, Some("Merged Planes"), storage_ro, 1),
            clip_boxes_buffer: WgpuVecBuffer::new(ctx, Some("Merged ClipBoxes"), storage_rw, 5),
            points3d_buffer: WgpuVecBuffer::new(ctx, Some("Merged Points3D"), storage_ro, 1),
            points2d_buffer: WgpuVecBuffer::new(ctx, Some("Merged Points2D"), storage_rw, 1),
            fill_rgbas_buffer: WgpuVecBuffer::new(ctx, Some("Merged FillRgbas"), storage_ro, 1),
            stroke_rgbas_buffer: WgpuVecBuffer::new(ctx, Some("Merged StrokeRgbas"), storage_ro, 1),
            stroke_widths_buffer: WgpuVecBuffer::new(
                ctx,
                Some("Merged StrokeWidths"),
                storage_ro,
                1,
            ),
            item_count: 0,
            total_points: 0,
            compute_bind_group: None,
            render_bind_group: None,
        }
    }

    /// Pack all VItems into the merged buffers. Called once per frame.
    pub fn update(&mut self, ctx: &WgpuContext, vitems: &[VItemRenderData]) {
        let item_count = vitems.iter().filter(|v| !v.points.is_empty()).count();
        if item_count == 0 {
            self.item_count = 0;
            self.total_points = 0;
            return;
        }

        // Pre-calculate total sizes
        let total_points: usize = vitems.iter().map(|v| v.points.len()).sum();
        let total_attrs: usize = vitems.iter().map(VItemRenderData::attr_count).sum();

        // Build index table and collect data
        let mut item_infos = Vec::with_capacity(item_count);
        let mut planes = Vec::with_capacity(item_count);
        let mut all_points3d = Vec::with_capacity(total_points);
        let mut all_fill_rgbas = Vec::with_capacity(total_attrs);
        let mut all_stroke_rgbas = Vec::with_capacity(total_attrs);
        let mut all_stroke_widths = Vec::with_capacity(total_attrs);

        let mut point_offset: u32 = 0;
        let mut attr_offset: u32 = 0;

        for vitem in vitems.iter().filter(|v| !v.points.is_empty()) {
            let render_points = &vitem.points;
            let pc = render_points.len() as u32;
            let ac = vitem.attr_count();

            item_infos.push(ItemInfo {
                point_offset,
                point_count: pc,
                attr_offset,
                attr_count: ac as u32,
            });

            let normal = vitem
                .normal
                .map(|n| n)
                .unwrap_or_else(|| vitem_normal_from_points(render_points));
            let origin = Vec3::new(render_points[0].x, render_points[0].y, render_points[0].z);
            planes.push(PlaneData {
                normal: Vec4::from((normal, 0.0)),
                origin: Vec4::from((origin, 0.0)),
            });

            all_points3d.extend_from_slice(render_points);
            all_fill_rgbas.extend(resize_vec4_by_sample(&vitem.fill_rgbas, ac));
            all_stroke_rgbas.extend(resize_vec4_by_sample(&vitem.stroke_rgbas, ac));
            all_stroke_widths.extend(resize_f32_by_sample(&vitem.stroke_widths, ac));

            point_offset += pc;
            attr_offset += ac as u32;
        }

        // Build clip_boxes initial values: [MAX, MIN, MAX, MIN, 0] per item
        let mut clip_boxes = Vec::with_capacity(item_count * 5);
        for _ in 0..item_count {
            clip_boxes.extend_from_slice(&[i32::MAX, i32::MIN, i32::MAX, i32::MIN, 0]);
        }

        // Points2d: zeroed, same size as points3d
        let points2d = vec![Vec4::ZERO; total_points];

        self.item_count = item_count as u32;
        self.total_points = total_points as u32;

        // Upload all data — track if any buffer was reallocated
        let mut any_realloc = false;
        any_realloc |= self.item_infos_buffer.set(ctx, &item_infos);
        any_realloc |= self.planes_buffer.set(ctx, &planes);
        any_realloc |= self.clip_boxes_buffer.set(ctx, &clip_boxes);
        any_realloc |= self.points3d_buffer.set(ctx, &all_points3d);
        any_realloc |= self.points2d_buffer.set(ctx, &points2d);
        any_realloc |= self.fill_rgbas_buffer.set(ctx, &all_fill_rgbas);
        any_realloc |= self.stroke_rgbas_buffer.set(ctx, &all_stroke_rgbas);
        any_realloc |= self.stroke_widths_buffer.set(ctx, &all_stroke_widths);

        // Recreate bind groups if any buffer was reallocated
        if any_realloc || self.compute_bind_group.is_none() {
            self.compute_bind_group = Some(Self::create_compute_bind_group(ctx, self));
            self.render_bind_group = Some(Self::create_render_bind_group(ctx, self));
        }
    }

    pub fn item_count(&self) -> u32 {
        self.item_count
    }

    pub fn total_points(&self) -> u32 {
        self.total_points
    }

    // MARK: Bind group layouts

    pub fn compute_bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Merged VItem Compute BGL"),
                entries: &[
                    // binding 0: item_infos (read-only)
                    bgl_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    // binding 1: planes (read-only)
                    bgl_entry(1, wgpu::ShaderStages::COMPUTE, false),
                    // binding 2: points3d (read-only)
                    bgl_entry(2, wgpu::ShaderStages::COMPUTE, false),
                    // binding 3: stroke_widths (read-only)
                    bgl_entry(3, wgpu::ShaderStages::COMPUTE, false),
                    // binding 4: points2d (read-write)
                    bgl_entry(4, wgpu::ShaderStages::COMPUTE, true),
                    // binding 5: clip_boxes (read-write)
                    bgl_entry(5, wgpu::ShaderStages::COMPUTE, true),
                ],
            })
    }

    pub fn render_bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        let vf = wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT;
        let v = wgpu::ShaderStages::VERTEX;
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Merged VItem Render BGL"),
                entries: &[
                    // binding 0: item_infos
                    bgl_entry(0, vf, false),
                    // binding 1: planes (normal + origin)
                    bgl_entry(1, v, false),
                    // binding 2: clip_boxes
                    bgl_entry(2, v, false),
                    // binding 3: points2d
                    bgl_entry(3, vf, false),
                    // binding 4: fill_rgbas
                    bgl_entry(4, vf, false),
                    // binding 5: stroke_rgbas
                    bgl_entry(5, vf, false),
                    // binding 6: stroke_widths
                    bgl_entry(6, vf, false),
                ],
            })
    }

    fn create_compute_bind_group(ctx: &WgpuContext, this: &Self) -> wgpu::BindGroup {
        ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Merged VItem Compute BG"),
            layout: &Self::compute_bind_group_layout(ctx),
            entries: &[
                bg_entry(0, &this.item_infos_buffer.buffer),
                bg_entry(1, &this.planes_buffer.buffer),
                bg_entry(2, &this.points3d_buffer.buffer),
                bg_entry(3, &this.stroke_widths_buffer.buffer),
                bg_entry(4, &this.points2d_buffer.buffer),
                bg_entry(5, &this.clip_boxes_buffer.buffer),
            ],
        })
    }

    fn create_render_bind_group(ctx: &WgpuContext, this: &Self) -> wgpu::BindGroup {
        ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Merged VItem Render BG"),
            layout: &Self::render_bind_group_layout(ctx),
            entries: &[
                bg_entry(0, &this.item_infos_buffer.buffer),
                bg_entry(1, &this.planes_buffer.buffer),
                bg_entry(2, &this.clip_boxes_buffer.buffer),
                bg_entry(3, &this.points2d_buffer.buffer),
                bg_entry(4, &this.fill_rgbas_buffer.buffer),
                bg_entry(5, &this.stroke_rgbas_buffer.buffer),
                bg_entry(6, &this.stroke_widths_buffer.buffer),
            ],
        })
    }
}

/// Compute the normal vector from rendered VItem points.
///
/// Falls back to Z axis if the first three points are collinear.
pub fn vitem_normal_from_points(points: &[Vec4]) -> Vec3 {
    if points.len() < 3 {
        return Vec3::Z;
    }
    let p0 = Vec3::new(points[0].x, points[0].y, points[0].z);
    let p1 = Vec3::new(points[1].x, points[1].y, points[1].z);
    let p2 = Vec3::new(points[2].x, points[2].y, points[2].z);
    let n = (p1 - p0).cross(p2 - p0);
    if n.length_squared() < 1e-6 {
        Vec3::Z
    } else {
        n.normalize()
    }
}

fn resize_vec4_by_sample(values: &[Vec4], len: usize) -> Vec<Vec4> {
    (0..len).map(|idx| sample_vec4(values, idx, len)).collect()
}

fn sample_vec4(values: &[Vec4], idx: usize, total: usize) -> Vec4 {
    match values.len() {
        0 => Vec4::ZERO,
        1 => values[0],
        len => {
            if total <= 1 {
                return values[0];
            }
            let t = idx as f32 / (total - 1) as f32;
            let pos = t * (len - 1) as f32;
            let lo = (pos.floor() as usize).min(len - 2);
            let frac = pos - lo as f32;
            values[lo] + (values[lo + 1] - values[lo]) * frac
        }
    }
}

fn resize_f32_by_sample(values: &[f32], len: usize) -> Vec<f32> {
    (0..len).map(|idx| sample_f32(values, idx, len)).collect()
}

fn sample_f32(values: &[f32], idx: usize, total: usize) -> f32 {
    match values.len() {
        0 => 0.0,
        1 => values[0],
        len => {
            if total <= 1 {
                return values[0];
            }
            let t = idx as f32 / (total - 1) as f32;
            let pos = t * (len - 1) as f32;
            let lo = (pos.floor() as usize).min(len - 2);
            let frac = pos - lo as f32;
            values[lo] + (values[lo + 1] - values[lo]) * frac
        }
    }
}

fn bgl_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    read_write: bool,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage {
                read_only: !read_write,
            },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bg_entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::Buffer(buffer.as_entire_buffer_binding()),
    }
}
