use crate::utils::{WgpuContext, WgpuVecBuffer};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use ranim_core::{components::rgba::Rgba, core_item::mesh_item::MeshItem};

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct MeshTransform {
    pub transform: [[f32; 4]; 4],
}

pub struct MeshItemsBuffer {
    /// Per-vertex positions (vertex buffer)
    pub(crate) vertices_buffer: WgpuVecBuffer<Vec3>,
    /// Per-vertex mesh id (vertex buffer)
    pub(crate) mesh_ids_buffer: WgpuVecBuffer<u32>,
    /// Merged triangle indices (index buffer)
    pub(crate) indices_buffer: WgpuVecBuffer<u32>,

    /// Per-mesh transform matrices (storage buffer, indexed by mesh_id)
    pub(crate) transforms_buffer: WgpuVecBuffer<MeshTransform>,
    /// Per-mesh fill colors (storage buffer, indexed by mesh_id)
    pub(crate) fill_rgbas_buffer: WgpuVecBuffer<Rgba>,

    pub(crate) item_count: u32,
    pub(crate) total_vertices: u32,
    pub(crate) total_indices: u32,

    pub(crate) render_bind_group: Option<wgpu::BindGroup>,
}

impl MeshItemsBuffer {
    pub fn new(ctx: &WgpuContext) -> Self {
        let vertex_usage = wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST;
        let index_usage = wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST;
        let storage_ro = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;

        Self {
            vertices_buffer: WgpuVecBuffer::new(ctx, Some("MeshVertices"), vertex_usage, 1),
            mesh_ids_buffer: WgpuVecBuffer::new(ctx, Some("MeshIds"), vertex_usage, 1),
            indices_buffer: WgpuVecBuffer::new(ctx, Some("MeshIndices"), index_usage, 1),
            transforms_buffer: WgpuVecBuffer::new(ctx, Some("MeshTransforms"), storage_ro, 1),
            fill_rgbas_buffer: WgpuVecBuffer::new(ctx, Some("MeshFillRgbas"), storage_ro, 1),
            item_count: 0,
            total_vertices: 0,
            total_indices: 0,
            render_bind_group: None,
        }
    }

    pub fn update(&mut self, ctx: &WgpuContext, mesh_items: &[MeshItem]) {
        if mesh_items.is_empty() {
            self.item_count = 0;
            self.total_vertices = 0;
            self.total_indices = 0;
            return;
        }

        let item_count = mesh_items.len();
        let total_vertices: usize = mesh_items.iter().map(|m| m.points.len()).sum();
        let total_indices: usize = mesh_items.iter().map(|m| m.triangle_indices.len()).sum();

        let mut transforms = Vec::with_capacity(item_count);
        let mut all_vertices = Vec::with_capacity(total_vertices);
        let mut all_mesh_ids = Vec::with_capacity(total_vertices);
        let mut all_indices = Vec::with_capacity(total_indices);
        let mut all_fill_rgbas = Vec::with_capacity(item_count);

        let mut vertex_offset: u32 = 0;

        for (mesh_idx, mesh) in mesh_items.iter().enumerate() {
            let vc = mesh.points.len() as u32;

            transforms.push(MeshTransform {
                transform: mesh.transform.to_cols_array_2d(),
            });

            all_vertices.extend_from_slice(&mesh.points);
            all_mesh_ids.extend(std::iter::repeat(mesh_idx as u32).take(vc as usize));
            all_indices.extend(mesh.triangle_indices.iter().map(|&i| i + vertex_offset));
            all_fill_rgbas.push(mesh.fill_rgba);

            vertex_offset += vc;
        }

        self.item_count = item_count as u32;
        self.total_vertices = total_vertices as u32;
        self.total_indices = total_indices as u32;

        // Vertex/index buffers (no bind group dependency)
        self.vertices_buffer.set(ctx, &all_vertices);
        self.mesh_ids_buffer.set(ctx, &all_mesh_ids);
        self.indices_buffer.set(ctx, &all_indices);

        // Storage buffers (bind group recreated on realloc)
        let mut any_realloc = false;
        any_realloc |= self.transforms_buffer.set(ctx, &transforms);
        any_realloc |= self.fill_rgbas_buffer.set(ctx, &all_fill_rgbas);

        if any_realloc || self.render_bind_group.is_none() {
            self.render_bind_group = Some(Self::create_render_bind_group(ctx, self));
        }
    }

    pub fn item_count(&self) -> u32 {
        self.item_count
    }

    pub fn total_indices(&self) -> u32 {
        self.total_indices
    }

    pub fn vertex_buffer_layouts() -> [wgpu::VertexBufferLayout<'static>; 2] {
        [
            // Slot 0: positions (vec3<f32>)
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vec3>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                }],
            },
            // Slot 1: mesh_id (u32)
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<u32>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Uint32,
                    offset: 0,
                    shader_location: 1,
                }],
            },
        ]
    }

    pub fn render_bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MeshItems Render BGL"),
                entries: &[
                    // binding 0: transforms (per-mesh, vertex stage)
                    bgl_storage_entry(0, wgpu::ShaderStages::VERTEX),
                    // binding 1: fill_rgbas (per-mesh, fragment stage)
                    bgl_storage_entry(1, wgpu::ShaderStages::FRAGMENT),
                ],
            })
    }

    fn create_render_bind_group(ctx: &WgpuContext, this: &Self) -> wgpu::BindGroup {
        ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MeshItems Render BG"),
            layout: &Self::render_bind_group_layout(ctx),
            entries: &[
                bg_entry(0, &this.transforms_buffer.buffer),
                bg_entry(1, &this.fill_rgbas_buffer.buffer),
            ],
        })
    }
}

fn bgl_storage_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
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
    use std::path::Path;

    use super::*;
    use crate::{Renderer, resource::RenderPool};
    use glam::{Mat4, Vec3};
    use pollster::block_on;
    use ranim_core::{components::rgba::Rgba, core_item::CoreItem, store::CoreItemStore};

    fn create_triangle_mesh(color: Rgba, offset: Vec3) -> MeshItem {
        MeshItem {
            points: vec![
                Vec3::new(0.0, 1.0, 0.0) + offset,
                Vec3::new(-1.0, -1.0, 0.0) + offset,
                Vec3::new(1.0, -1.0, 0.0) + offset,
            ],
            triangle_indices: vec![0, 1, 2],
            transform: Mat4::IDENTITY,
            fill_rgba: color,
        }
    }

    fn create_quad_mesh(color: Rgba, offset: Vec3) -> MeshItem {
        MeshItem {
            points: vec![
                Vec3::new(-1.0, 1.0, 0.0) + offset,
                Vec3::new(1.0, 1.0, 0.0) + offset,
                Vec3::new(1.0, -1.0, 0.0) + offset,
                Vec3::new(-1.0, -1.0, 0.0) + offset,
            ],
            triangle_indices: vec![0, 1, 2, 0, 2, 3],
            transform: Mat4::IDENTITY,
            fill_rgba: color,
        }
    }

    #[test]
    fn render_mesh_items() {
        use ranim_core::core_item::camera_frame::CameraFrame;

        let ctx = block_on(WgpuContext::new());

        let width = 800u32;
        let height = 600u32;

        let mut renderer = Renderer::new(&ctx, width, height, 8);
        let mut render_textures = renderer.new_render_textures(&ctx);
        let mut pool = RenderPool::new();

        let mut store = CoreItemStore::new();

        let red = Rgba(glam::Vec4::new(1.0, 0.0, 0.0, 1.0));
        let green = Rgba(glam::Vec4::new(0.0, 1.0, 0.0, 1.0));
        let blue = Rgba(glam::Vec4::new(0.0, 0.0, 1.0, 0.8));
        let yellow = Rgba(glam::Vec4::new(1.0, 1.0, 0.0, 0.9));

        let camera_frame = CameraFrame::default();
        let triangle1 = create_triangle_mesh(red, Vec3::new(-2.0, 0.0, 0.0));
        let triangle2 = create_triangle_mesh(green, Vec3::new(2.0, 0.0, 0.0));
        let quad1 = create_quad_mesh(blue, Vec3::new(0.0, 2.0, 0.0));
        let quad2 = create_quad_mesh(yellow, Vec3::new(0.0, -2.0, 0.0));

        store.update(
            [
                ((0, 0), CoreItem::CameraFrame(camera_frame)),
                ((1, 0), CoreItem::MeshItem(triangle1)),
                ((1, 1), CoreItem::MeshItem(triangle2)),
                ((2, 0), CoreItem::MeshItem(quad1)),
                ((3, 1), CoreItem::MeshItem(quad2)),
            ]
            .into_iter(),
        );

        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.1,
            b: 0.1,
            a: 1.0,
        };

        renderer.render_store_with_pool(&ctx, &mut render_textures, clear_color, &store, &mut pool);
        pool.clean();

        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        let buffer = render_textures.get_rendered_texture_img_buffer(&ctx);

        let output_path = Path::new("../../output/mesh_items_render.png");
        buffer.save(output_path).expect("Failed to save image");

        println!("Rendered image saved to: {:?}", output_path);
        println!("Open it to see the mesh rendering result!");

        assert!(output_path.exists(), "Image file should be created");
    }
}
