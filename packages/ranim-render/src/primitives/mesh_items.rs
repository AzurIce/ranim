use crate::utils::{WgpuContext, WgpuVecBuffer};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use ranim_core::{components::rgba::Rgba, core_item::mesh_item::MeshItem};

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct MeshTransform {
    pub transform: [[f32; 4]; 4],
}

#[derive(bevy_ecs::prelude::Resource)]
pub struct MeshItemsBuffer {
    /// Per-vertex positions (vertex buffer)
    pub(crate) vertices_buffer: WgpuVecBuffer<Vec3>,
    /// Per-vertex mesh id (vertex buffer)
    pub(crate) mesh_ids_buffer: WgpuVecBuffer<u32>,
    /// Per-vertex colors (vertex buffer)
    pub(crate) vertex_colors_buffer: WgpuVecBuffer<Rgba>,
    /// Per-vertex normals (vertex buffer) — all-zero → flat shading fallback
    pub(crate) vertex_normals_buffer: WgpuVecBuffer<Vec3>,
    /// Merged triangle indices (index buffer)
    pub(crate) indices_buffer: WgpuVecBuffer<u32>,

    /// Per-mesh transform matrices (storage buffer, indexed by mesh_id)
    pub(crate) transforms_buffer: WgpuVecBuffer<MeshTransform>,
    /// Per-mesh global scene order (storage buffer, indexed by mesh_id),
    /// consumed by the fragment depth-order bias.
    pub(crate) orders_buffer: WgpuVecBuffer<f32>,

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
            vertex_colors_buffer: WgpuVecBuffer::new(
                ctx,
                Some("MeshVertexColors"),
                vertex_usage,
                1,
            ),
            vertex_normals_buffer: WgpuVecBuffer::new(
                ctx,
                Some("MeshVertexNormals"),
                vertex_usage,
                1,
            ),
            indices_buffer: WgpuVecBuffer::new(ctx, Some("MeshIndices"), index_usage, 1),
            transforms_buffer: WgpuVecBuffer::new(ctx, Some("MeshTransforms"), storage_ro, 1),
            orders_buffer: WgpuVecBuffer::new(ctx, Some("MeshOrders"), storage_ro, 1),
            item_count: 0,
            total_vertices: 0,
            total_indices: 0,
            render_bind_group: None,
        }
    }

    pub fn update<'a, I>(&mut self, ctx: &WgpuContext, mesh_items: I)
    where
        I: IntoIterator<Item = (f32, &'a MeshItem)>,
        I::IntoIter: ExactSizeIterator + Clone,
    {
        let mesh_items = mesh_items.into_iter();
        if mesh_items.len() == 0 {
            self.item_count = 0;
            self.total_vertices = 0;
            self.total_indices = 0;
            return;
        }

        let item_count = mesh_items.len();
        let total_vertices: usize = mesh_items.clone().map(|(_, m)| m.points.len()).sum();
        let total_indices: usize = mesh_items
            .clone()
            .map(|(_, m)| m.triangle_indices.len())
            .sum();

        let mut transforms = Vec::with_capacity(item_count);
        let mut all_orders = Vec::with_capacity(item_count);
        let mut all_vertices = Vec::with_capacity(total_vertices);
        let mut all_mesh_ids = Vec::with_capacity(total_vertices);
        let mut all_vertex_colors = Vec::with_capacity(total_vertices);
        let mut all_vertex_normals = Vec::with_capacity(total_vertices);
        let mut all_indices = Vec::with_capacity(total_indices);

        let mut vertex_offset: u32 = 0;

        for (mesh_idx, (order, mesh)) in mesh_items.enumerate() {
            let vc = mesh.points.len() as u32;

            transforms.push(MeshTransform {
                transform: mesh.transform.to_cols_array_2d(),
            });
            all_orders.push(order);

            all_vertices.extend_from_slice(&mesh.points);
            all_mesh_ids.extend(std::iter::repeat_n(mesh_idx as u32, vc as usize));
            all_vertex_colors.extend_from_slice(&mesh.vertex_colors);

            // Pad normals with zero if shorter than points (flat shading fallback)
            let normals = &mesh.vertex_normals;
            let normals_len = normals.len();
            if normals_len >= vc as usize {
                all_vertex_normals.extend_from_slice(&normals[..vc as usize]);
            } else {
                all_vertex_normals.extend_from_slice(normals);
                all_vertex_normals
                    .extend(std::iter::repeat_n(Vec3::ZERO, vc as usize - normals_len));
            }

            all_indices.extend(mesh.triangle_indices.iter().map(|&i| i + vertex_offset));

            vertex_offset += vc;
        }

        self.item_count = item_count as u32;
        self.total_vertices = total_vertices as u32;
        self.total_indices = total_indices as u32;

        // Vertex/index buffers (no bind group dependency)
        self.vertices_buffer.set(ctx, &all_vertices);
        self.mesh_ids_buffer.set(ctx, &all_mesh_ids);
        self.vertex_colors_buffer.set(ctx, &all_vertex_colors);
        self.vertex_normals_buffer.set(ctx, &all_vertex_normals);
        self.indices_buffer.set(ctx, &all_indices);

        // Storage buffers (bind group recreated on realloc)
        let mut any_realloc = self.transforms_buffer.set(ctx, &transforms);
        any_realloc |= self.orders_buffer.set(ctx, &all_orders);

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

    pub fn vertex_buffer_layouts() -> [wgpu::VertexBufferLayout<'static>; 4] {
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
            // Slot 2: vertex_color (vec4<f32>)
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Rgba>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 2,
                }],
            },
            // Slot 3: vertex_normal (vec3<f32>)
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vec3>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 3,
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
                    // binding 1: orders (per-mesh, fragment stage, depth bias)
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
                bg_entry(1, &this.orders_buffer.buffer),
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
