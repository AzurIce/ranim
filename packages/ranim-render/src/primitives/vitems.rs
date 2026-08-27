use crate::utils::{WgpuContext, WgpuVecBuffer};
use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};
use ranim_core::{
    components::{rgba::Rgba, width::Width},
    core_item::vitem::{VItem, vitem_normal_from_points},
};

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

/// Per-item local-to-world transform, applied by the vertex stage after
/// reconstructing the 3D position from the plane basis.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VitemTransform {
    pub transform: [[f32; 4]; 4],
}

impl Default for VitemTransform {
    fn default() -> Self {
        Self {
            transform: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
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
#[derive(bevy_ecs::prelude::Resource)]
pub struct VItemsBuffer {
    /// Per-item metadata: offsets and counts
    pub(crate) item_infos_buffer: WgpuVecBuffer<ItemInfo>,
    /// Per-item plane data (normal + origin for vertex shader)
    pub(crate) planes_buffer: WgpuVecBuffer<PlaneData>,
    /// Per-item clip boxes (5 i32 each: min_x, max_x, min_y, max_y, max_w)
    pub(crate) clip_boxes_buffer: WgpuVecBuffer<i32>,
    /// Per-item local-to-world transforms
    pub(crate) transforms_buffer: WgpuVecBuffer<VitemTransform>,

    /// Merged 3D points from all VItems
    pub(crate) points3d_buffer: WgpuVecBuffer<Vec4>,
    /// Merged 2D projected points (written by compute shader)
    pub(crate) points2d_buffer: WgpuVecBuffer<Vec4>,
    /// Merged fill colors
    pub(crate) fill_rgbas_buffer: WgpuVecBuffer<Rgba>,
    /// Merged stroke colors
    pub(crate) stroke_rgbas_buffer: WgpuVecBuffer<Rgba>,
    /// Merged stroke widths
    pub(crate) stroke_widths_buffer: WgpuVecBuffer<Width>,

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
            transforms_buffer: WgpuVecBuffer::new(
                ctx,
                Some("Merged VItem Transforms"),
                storage_ro,
                1,
            ),
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
    pub fn update<'a, I>(&mut self, ctx: &WgpuContext, vitems: I)
    where
        I: IntoIterator<Item = (f32, &'a VItem)>,
        I::IntoIter: ExactSizeIterator + Clone,
    {
        let vitems = vitems.into_iter();
        if vitems.len() == 0 {
            self.item_count = 0;
            self.total_points = 0;
            return;
        }

        let item_count = vitems.len();

        // Pre-calculate total sizes
        let total_points: usize = vitems.clone().map(|(_, v)| v.points.len()).sum();
        let total_attrs: usize = vitems
            .clone()
            .map(|(_, v)| v.points.len().div_ceil(2))
            .sum();

        // Build index table and collect data
        let mut item_infos = Vec::with_capacity(item_count);
        let mut planes = Vec::with_capacity(item_count);
        let mut transforms = Vec::with_capacity(item_count);
        let mut all_points3d = Vec::with_capacity(total_points);
        let mut all_fill_rgbas = Vec::with_capacity(total_attrs);
        let mut all_stroke_rgbas = Vec::with_capacity(total_attrs);
        let mut all_stroke_widths = Vec::with_capacity(total_attrs);

        let mut point_offset: u32 = 0;
        let mut attr_offset: u32 = 0;

        for (order, vitem) in vitems {
            let pc = vitem.points.len() as u32;
            let ac = pc.div_ceil(2);

            item_infos.push(ItemInfo {
                point_offset,
                point_count: pc,
                attr_offset,
                attr_count: ac,
            });

            let normal = vitem
                .normal
                .unwrap_or_else(|| vitem_normal_from_points(&vitem.points));
            let origin = Vec3::new(vitem.points[0].x, vitem.points[0].y, vitem.points[0].z);
            planes.push(PlaneData {
                normal: Vec4::from((normal, 0.0)),
                // origin.w carries the item's global scene order for the
                // depth-order bias; the plane basis ignores it.
                origin: Vec4::from((origin, order)),
            });

            all_points3d.extend_from_slice(&vitem.points);
            transforms.push(VitemTransform {
                transform: vitem.transform.to_cols_array_2d(),
            });
            all_fill_rgbas.extend_from_slice(&vitem.fill_rgbas);
            all_stroke_rgbas.extend_from_slice(&vitem.stroke_rgbas);
            all_stroke_widths.extend_from_slice(&vitem.stroke_widths);

            point_offset += pc;
            attr_offset += ac;
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
        any_realloc |= self.transforms_buffer.set(ctx, &transforms);
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
                    // binding 1: planes (normal + origin; origin.w = scene order)
                    bgl_entry(1, vf, false),
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
                    // binding 7: per-item local-to-world transforms
                    bgl_entry(7, v, false),
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
                bg_entry(7, &this.transforms_buffer.buffer),
            ],
        })
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::{Renderer, world::RenderFrame};
    use glam::{Mat4, Vec3};
    use pollster::block_on;
    use ranim_core::{
        components::rgba::Rgba,
        core_item::{CoreItem, camera_frame::CameraFrame},
    };

    fn test_output_path(filename: &str) -> PathBuf {
        let output_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../output");
        std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");
        output_dir.join(filename)
    }

    /// A closed unit square centered at the origin in local space.
    fn square_vitem(color: Rgba, stroke: Rgba) -> VItem {
        let p = |x: f32, y: f32| Vec4::new(x, y, 0.0, 1.0);
        let mid = |a: Vec4, b: Vec4| Vec4::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5, 0.0, 1.0);
        let a0 = p(-0.5, -0.5);
        let a1 = p(0.5, -0.5);
        let a2 = p(0.5, 0.5);
        let a3 = p(-0.5, 0.5);
        VItem {
            normal: None,
            points: vec![
                a0,
                mid(a0, a1),
                a1,
                mid(a1, a2),
                a2,
                mid(a2, a3),
                a3,
                mid(a3, a0),
            ],
            transform: Mat4::IDENTITY,
            fill_rgbas: vec![color; 4],
            stroke_rgbas: vec![stroke; 4],
            stroke_widths: vec![ranim_core::components::width::Width(0.02); 4],
        }
    }

    #[test]
    fn render_transformed_vitems() {
        let ctx = block_on(WgpuContext::new());
        let width = 800u32;
        let height = 600u32;

        let mut renderer = Renderer::new(&ctx, width, height, 8);
        let mut render_textures = renderer.new_render_textures(&ctx);
        let mut store = RenderFrame::new();

        // The red square is rotated around Z, the blue one scaled; both keep
        // identity local points and are placed only through `transform`.
        let mut red = square_vitem(
            Rgba(glam::Vec4::new(1.0, 0.0, 0.0, 0.6)),
            Rgba(glam::Vec4::new(1.0, 1.0, 1.0, 1.0)),
        );
        red.transform = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_4)
            * Mat4::from_translation(Vec3::new(2.0, 0.0, 0.0));
        let mut blue = square_vitem(
            Rgba(glam::Vec4::new(0.0, 0.0, 1.0, 0.6)),
            Rgba(glam::Vec4::new(1.0, 1.0, 1.0, 1.0)),
        );
        blue.transform =
            Mat4::from_scale(Vec3::splat(2.0)) * Mat4::from_translation(Vec3::new(-2.5, 0.0, 0.0));

        store.update(
            [
                ((0, 0), CoreItem::CameraFrame(CameraFrame::default())),
                ((1, 0), CoreItem::VItem(red)),
                ((1, 1), CoreItem::VItem(blue)),
            ]
            .into_iter(),
        );

        renderer.render_frame(&mut render_textures, wgpu::Color::BLACK, &store);
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        let buffer = render_textures.get_rendered_texture_img_buffer(&ctx);
        let output_path = test_output_path("vitems_transformed_render.png");
        buffer.save(&output_path).expect("Failed to save image");
        assert!(output_path.exists(), "Image file should be created");
    }
}
