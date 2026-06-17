use crate::utils::{WgpuContext, WgpuVecBuffer};
use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};

use crate::scene::MeshRenderData;

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
    pub(crate) vertex_colors_buffer: WgpuVecBuffer<Vec4>,
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

    pub fn update(&mut self, ctx: &WgpuContext, mesh_items: &[MeshRenderData]) {
        let item_count = mesh_items.iter().filter(|m| !m.points.is_empty()).count();
        if item_count == 0 {
            self.item_count = 0;
            self.total_vertices = 0;
            self.total_indices = 0;
            return;
        }

        let total_vertices: usize = mesh_items.iter().map(|m| m.points.len()).sum();
        let total_indices: usize = mesh_items.iter().map(|m| m.indices.len()).sum();

        let mut transforms = Vec::with_capacity(item_count);
        let mut all_vertices = Vec::with_capacity(total_vertices);
        let mut all_mesh_ids = Vec::with_capacity(total_vertices);
        let mut all_vertex_colors = Vec::with_capacity(total_vertices);
        let mut all_vertex_normals = Vec::with_capacity(total_vertices);
        let mut all_indices = Vec::with_capacity(total_indices);

        let mut vertex_offset: u32 = 0;

        for (mesh_idx, mesh) in mesh_items
            .iter()
            .filter(|m| !m.points.is_empty())
            .enumerate()
        {
            let vc = mesh.points.len() as u32;

            transforms.push(MeshTransform {
                transform: mesh.transform.to_cols_array_2d(),
            });

            all_vertices.extend_from_slice(&mesh.points[..]);
            all_mesh_ids.extend(std::iter::repeat_n(mesh_idx as u32, vc as usize));
            all_vertex_colors.extend(resize_vec4_by_sample(&mesh.vertex_colors, vc as usize));

            // Pad normals with zero if shorter than points (flat shading fallback)
            let normals = &mesh.vertex_normals;
            let normals_len = normals.len();
            if normals_len >= vc as usize {
                all_vertex_normals.extend_from_slice(&normals[..vc as usize]);
            } else {
                all_vertex_normals.extend_from_slice(&normals[..]);
                all_vertex_normals
                    .extend(std::iter::repeat_n(Vec3::ZERO, vc as usize - normals_len));
            }

            all_indices.extend(mesh.indices.iter().map(|&i| i + vertex_offset));

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
                array_stride: std::mem::size_of::<Vec4>() as u64,
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

fn resize_vec4_by_sample(values: &[Vec4], len: usize) -> Vec<Vec4> {
    (0..len).map(|idx| sample_vec4(values, idx, len)).collect()
}

fn sample_vec4(values: &[Vec4], idx: usize, total: usize) -> Vec4 {
    match values.len() {
        0 => Vec4::ONE,
        1 => values[0],
        len => {
            if total <= 1 {
                return values[0];
            }
            let t = idx as f32 / (total - 1) as f32;
            let pos = t * (len - 1) as f32;
            let lo = (pos.floor() as usize).min(len - 2);
            values[lo].lerp(values[lo + 1], pos - lo as f32)
        }
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
    use crate::{
        Renderer,
        scene::{RenderScene, ViewData},
    };
    use glam::{Mat4, Vec2, Vec3, Vec4};
    use pollster::block_on;

    fn test_view(width: u32, height: u32) -> ViewData {
        let ratio = width as f32 / height as f32;
        let frame_height = 8.0;
        let frame_width = frame_height * ratio;
        ViewData {
            proj_mat: Mat4::orthographic_rh(
                -frame_width / 2.0,
                frame_width / 2.0,
                -frame_height / 2.0,
                frame_height / 2.0,
                -1000.0,
                1000.0,
            ),
            view_mat: Mat4::look_to_rh(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y),
            half_frame_size: Vec2::new(frame_width / 2.0, frame_height / 2.0),
        }
    }

    fn create_triangle_mesh(color: Vec4, offset: Vec3) -> MeshRenderData {
        MeshRenderData {
            points: vec![
                Vec3::new(0.0, 1.0, 0.0) + offset,
                Vec3::new(-1.0, -1.0, 0.0) + offset,
                Vec3::new(1.0, -1.0, 0.0) + offset,
            ],
            indices: vec![0, 1, 2],
            transform: Mat4::IDENTITY,
            vertex_colors: vec![color; 3],
            vertex_normals: vec![Vec3::ZERO; 3],
        }
    }

    fn create_quad_mesh(color: Vec4, offset: Vec3) -> MeshRenderData {
        MeshRenderData {
            points: vec![
                Vec3::new(-1.0, 1.0, 0.0) + offset,
                Vec3::new(1.0, 1.0, 0.0) + offset,
                Vec3::new(1.0, -1.0, 0.0) + offset,
                Vec3::new(-1.0, -1.0, 0.0) + offset,
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            transform: Mat4::IDENTITY,
            vertex_colors: vec![color; 4],
            vertex_normals: vec![Vec3::ZERO; 4],
        }
    }

    fn create_sphere_mesh(color: Vec4, radius: f32, position: Vec3) -> MeshRenderData {
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
        let vertex_normals = points
            .iter()
            .map(|p| (*p - position).normalize())
            .collect::<Vec<_>>();

        MeshRenderData {
            points,
            indices,
            transform: Mat4::IDENTITY,
            vertex_colors,
            vertex_normals,
        }
    }

    #[test]
    fn render_mesh_items() {
        let ctx = block_on(WgpuContext::new());

        let width = 800u32;
        let height = 600u32;

        let mut renderer = Renderer::new(&ctx, width, height, 8);
        let mut render_textures = renderer.new_render_textures(&ctx);

        let red = Vec4::new(1.0, 0.0, 0.0, 1.0);
        let green = Vec4::new(0.0, 1.0, 0.0, 1.0);
        let blue = Vec4::new(0.0, 0.0, 1.0, 0.8);
        let yellow = Vec4::new(1.0, 1.0, 0.0, 0.9);

        let triangle1 = create_triangle_mesh(red, Vec3::new(-2.0, 0.0, 0.0));
        let triangle2 = create_triangle_mesh(green, Vec3::new(2.0, 0.0, 0.0));
        let quad1 = create_quad_mesh(blue, Vec3::new(0.0, 2.0, 0.0));
        let quad2 = create_quad_mesh(yellow, Vec3::new(0.0, -2.0, 0.0));

        let scene = RenderScene {
            view: test_view(width, height),
            vitems: Vec::new(),
            meshes: vec![triangle1, triangle2, quad1, quad2],
        };

        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.1,
            b: 0.1,
            a: 1.0,
        };

        renderer.render_scene(&ctx, &mut render_textures, clear_color, &scene);

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
        let ctx = block_on(WgpuContext::new());
        let width = 800u32;
        let height = 600u32;

        let mut renderer = Renderer::new(&ctx, width, height, 8);
        let mut render_textures = renderer.new_render_textures(&ctx);

        // Create nested spheres:
        // 1. Outer transparent sphere (blue, alpha=0.3, radius=2.0)
        // 2. Middle opaque sphere (red, alpha=1.0, radius=1.5)
        // 3. Inner transparent sphere (green, alpha=0.5, radius=1.0)

        let outer_transparent = Vec4::new(0.0, 0.0, 1.0, 0.3);
        let middle_opaque = Vec4::new(1.0, 0.0, 0.0, 1.0);
        let inner_transparent = Vec4::new(0.0, 1.0, 0.0, 0.5);

        let outer_sphere = create_sphere_mesh(outer_transparent, 2.0, Vec3::ZERO);
        let middle_sphere = create_sphere_mesh(middle_opaque, 1.5, Vec3::ZERO);
        let inner_sphere = create_sphere_mesh(inner_transparent, 1.0, Vec3::ZERO);

        let scene = RenderScene {
            view: test_view(width, height),
            vitems: Vec::new(),
            meshes: vec![outer_sphere, middle_sphere, inner_sphere],
        };

        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.1,
            b: 0.1,
            a: 1.0,
        };

        renderer.render_scene(&ctx, &mut render_textures, clear_color, &scene);

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
