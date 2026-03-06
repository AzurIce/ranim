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
    /// Per-vertex colors (vertex buffer)
    pub(crate) vertex_colors_buffer: WgpuVecBuffer<Rgba>,
    /// Per-vertex normals (vertex buffer) — all-zero → flat shading fallback
    pub(crate) vertex_normals_buffer: WgpuVecBuffer<Vec3>,
    /// Merged triangle indices (index buffer)
    pub(crate) indices_buffer: WgpuVecBuffer<u32>,

    /// Per-mesh transform matrices (storage buffer, indexed by mesh_id)
    pub(crate) transforms_buffer: WgpuVecBuffer<MeshTransform>,

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
        let mut all_vertex_colors = Vec::with_capacity(total_vertices);
        let mut all_vertex_normals = Vec::with_capacity(total_vertices);
        let mut all_indices = Vec::with_capacity(total_indices);

        let mut vertex_offset: u32 = 0;

        for (mesh_idx, mesh) in mesh_items.iter().enumerate() {
            let vc = mesh.points.len() as u32;

            transforms.push(MeshTransform {
                transform: mesh.transform.to_cols_array_2d(),
            });

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
        let any_realloc = self.transforms_buffer.set(ctx, &transforms);

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
                ],
            })
    }

    fn create_render_bind_group(ctx: &WgpuContext, this: &Self) -> wgpu::BindGroup {
        ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MeshItems Render BG"),
            layout: &Self::render_bind_group_layout(ctx),
            entries: &[bg_entry(0, &this.transforms_buffer.buffer)],
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
            vertex_colors: vec![color; 3],
            vertex_normals: vec![Vec3::ZERO; 3],
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
            vertex_colors: vec![color; 4],
            vertex_normals: vec![Vec3::ZERO; 4],
        }
    }

    fn create_sphere_mesh(color: Rgba, radius: f32, position: Vec3) -> MeshItem {
        let mut points = Vec::new();
        let mut indices = Vec::new();

        // Simple UV sphere
        let lat_segments = 20;
        let lon_segments = 20;

        for lat in 0..=lat_segments {
            let theta = lat as f32 * std::f32::consts::PI / lat_segments as f32;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            for lon in 0..=lon_segments {
                let phi = lon as f32 * 2.0 * std::f32::consts::PI / lon_segments as f32;
                let sin_phi = phi.sin();
                let cos_phi = phi.cos();

                let x = sin_theta * cos_phi;
                let y = sin_theta * sin_phi;
                let z = cos_theta;

                points.push(Vec3::new(x * radius, y * radius, z * radius) + position);
            }
        }

        for lat in 0..lat_segments {
            for lon in 0..lon_segments {
                let first = lat * (lon_segments + 1) + lon;
                let second = first + lon_segments + 1;

                indices.push(first);
                indices.push(second);
                indices.push(first + 1);

                indices.push(second);
                indices.push(second + 1);
                indices.push(first + 1);
            }
        }

        let vertex_colors = vec![color; points.len()];
        let vertex_normals = points.iter().map(|p| (*p - position).normalize()).collect();

        MeshItem {
            points,
            triangle_indices: indices,
            transform: Mat4::IDENTITY,
            vertex_colors,
            vertex_normals,
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

    #[test]
    fn test_nested_transparent_spheres() {
        use ranim_core::core_item::camera_frame::CameraFrame;

        let ctx = block_on(WgpuContext::new());
        let width = 800u32;
        let height = 600u32;

        let mut renderer = Renderer::new(&ctx, width, height, 8);
        let mut render_textures = renderer.new_render_textures(&ctx);
        let mut pool = RenderPool::new();
        let mut store = CoreItemStore::new();

        // Create nested spheres:
        // 1. Outer transparent sphere (blue, alpha=0.3, radius=2.0)
        // 2. Middle opaque sphere (red, alpha=1.0, radius=1.5)
        // 3. Inner transparent sphere (green, alpha=0.5, radius=1.0)

        let outer_transparent = Rgba(glam::Vec4::new(0.0, 0.0, 1.0, 0.3));
        let middle_opaque = Rgba(glam::Vec4::new(1.0, 0.0, 0.0, 1.0));
        let inner_transparent = Rgba(glam::Vec4::new(0.0, 1.0, 0.0, 0.5));

        let outer_sphere = create_sphere_mesh(outer_transparent, 2.0, Vec3::ZERO);
        let middle_sphere = create_sphere_mesh(middle_opaque, 1.5, Vec3::ZERO);
        let inner_sphere = create_sphere_mesh(inner_transparent, 1.0, Vec3::ZERO);

        let camera_frame = CameraFrame::default();

        store.update(
            [
                ((0, 0), CoreItem::CameraFrame(camera_frame)),
                ((1, 0), CoreItem::MeshItem(outer_sphere)),
                ((2, 0), CoreItem::MeshItem(middle_sphere)),
                ((3, 0), CoreItem::MeshItem(inner_sphere)),
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

        // Analyze depth buffer
        let depth_data = render_textures.get_depth_texture_data(&ctx);
        let mut min_depth = f32::MAX;
        let mut max_depth = f32::MIN;
        let mut depth_histogram: std::collections::HashMap<u32, usize> =
            std::collections::HashMap::new();

        for &d in depth_data {
            if (d - 1.0).abs() > 0.001 {
                min_depth = min_depth.min(d);
                max_depth = max_depth.max(d);
                let bucket = (d * 10000.0) as u32;
                *depth_histogram.entry(bucket).or_insert(0) += 1;
            }
        }

        println!("\n=== Nested Spheres Depth Test ===");
        println!("Depth buffer analysis:");
        println!("  Min depth: {}", min_depth);
        println!("  Max depth: {}", max_depth);
        println!("\nDepth histogram (top 10 buckets):");
        let mut buckets: Vec<_> = depth_histogram.iter().collect();
        buckets.sort_by_key(|(k, _)| *k);
        for (bucket, count) in buckets.iter().take(10) {
            println!(
                "    depth ~{:.4}: {} pixels",
                **bucket as f32 / 10000.0,
                count
            );
        }

        let buffer = render_textures.get_rendered_texture_img_buffer(&ctx);

        // Sample some pixels to see actual colors
        println!("\nColor samples (center region):");
        let center_x = width / 2;
        let center_y = height / 2;
        for dy in [-50, 0, 50].iter() {
            for dx in [-50, 0, 50].iter() {
                let x = (center_x as i32 + dx) as u32;
                let y = (center_y as i32 + dy) as u32;
                if x < width && y < height {
                    let pixel = buffer.get_pixel(x, y);
                    println!(
                        "  ({:3}, {:3}): R={:3} G={:3} B={:3} A={:3}",
                        dx, dy, pixel[0], pixel[1], pixel[2], pixel[3]
                    );
                }
            }
        }

        let buffer = render_textures.get_rendered_texture_img_buffer(&ctx);
        let output_path = Path::new("../../output/nested_spheres_render.png");
        buffer.save(output_path).expect("Failed to save image");

        let depth_buffer = render_textures.get_depth_texture_img_buffer(&ctx);
        let depth_path = Path::new("../../output/nested_spheres_depth.png");
        depth_buffer
            .save(depth_path)
            .expect("Failed to save depth image");

        println!("\nImages saved to output/");
        println!("\nExpected behavior:");
        println!("  - Outer transparent blue sphere should be visible");
        println!("  - Middle opaque red sphere should occlude inner green sphere");
        println!("  - Inner green sphere should NOT be visible from outside");
        println!("  - Depth buffer should show opaque red sphere's depth");

        assert!(output_path.exists(), "Image file should be created");
        assert!(depth_path.exists(), "Depth image file should be created");
    }
}
