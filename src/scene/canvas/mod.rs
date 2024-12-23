pub mod camera;
pub mod pipeline;

use std::ops::{Deref, DerefMut};

use camera::CanvasCamera;
use glam::{vec2, Vec2, Vec3};
use log::trace;
use pipeline::CanvasPipeline;

use crate::context::WgpuContext;
use crate::rabject::Vertex;
use crate::scene::entity::Entity;

use crate::scene::store::EntityStore;
use crate::utils::wgpu::WgpuBuffer;

use crate::scene::SceneCamera;

/// A canvas is basically a 2d scene with a camera
///
/// To create a canvas, use [`crate::scene::Scene::insert_new_canvas`]
///
/// # Coordinate System
///
/// The coordinate system of the canvas is as follows:
/// - The origin is at the top-left corner of the canvas
/// - The x-axis is to the right
/// - The y-axis is downwards
///
pub struct Canvas {
    center_point: Vec3,
    up_normal: Vec3,
    unit_normal: Vec3,
    camera: CanvasCamera,
    entities: EntityStore<CanvasCamera>,
    vertex_buffer: WgpuBuffer<CanvasVertex>,
}

impl Deref for Canvas {
    type Target = EntityStore<CanvasCamera>;
    fn deref(&self) -> &Self::Target {
        &self.entities
    }
}

impl DerefMut for Canvas {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entities
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CanvasVertex {
    position: Vec3,
    uv: Vec2,
    _padding: f32,
}

impl Vertex for CanvasVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<CanvasVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<Vec3>() as u64,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

impl Canvas {
    pub fn new(ctx: &WgpuContext, width: u32, height: u32) -> Self {
        let vertex_buffer = WgpuBuffer::new(
            ctx,
            (std::mem::size_of::<CanvasVertex>() * 4) as u64,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );

        let camera = CanvasCamera::new(ctx, width, height);
        Self {
            center_point: Vec3::ZERO,
            up_normal: Vec3::Y,
            unit_normal: Vec3::Z,
            camera,
            entities: Default::default(),
            vertex_buffer,
        }
    }
    pub fn vertices(&self) -> [CanvasVertex; 4] {
        let half_width = self.camera.viewport_width as f32 / 2.0;
        let half_height = self.camera.viewport_height as f32 / 2.0;
        let left_normal = self.unit_normal.cross(self.up_normal).normalize();
        let tl = self.center_point + left_normal * half_width + self.up_normal * half_height;
        let tr = self.center_point - left_normal * half_width + self.up_normal * half_height;
        let bl = self.center_point + left_normal * half_width - self.up_normal * half_height;
        let br = self.center_point - left_normal * half_width - self.up_normal * half_height;
        [
            CanvasVertex {
                position: tl,
                uv: vec2(0.0, 0.0),
                _padding: 0.0,
            },
            CanvasVertex {
                position: tr,
                uv: vec2(1.0, 0.0),
                _padding: 0.0,
            },
            CanvasVertex {
                position: bl,
                uv: vec2(0.0, 1.0),
                _padding: 0.0,
            },
            CanvasVertex {
                position: br,
                uv: vec2(1.0, 1.0),
                _padding: 0.0,
            },
        ]
    }
}

impl Entity for Canvas {
    type Renderer = SceneCamera;

    fn tick(&mut self, dt: f32) {
        for (_, rabject) in self.entities.iter_mut() {
            rabject.tick(dt);
        }
    }
    fn extract(&mut self) {
        for (_, rabject) in self.entities.iter_mut() {
            rabject.extract();
        }
    }
    fn prepare(&mut self, ctx: &crate::context::RanimContext) {
        for (_, rabject) in self.entities.iter_mut() {
            rabject.prepare(ctx);
        }
        let vertices = self.vertices();
        self.vertex_buffer
            .prepare_from_slice(&ctx.wgpu_ctx, &vertices);
    }
    fn render(&mut self, ctx: &mut crate::context::RanimContext, renderer: &mut Self::Renderer) {
        // trace!("[Canvas] rendering inner entities...");
        // First render inner entities to camera's render_texture
        self.camera.render(ctx, &mut self.entities);

        // Then render the canvas to 3d scene
        // trace!("[Canvas] rendering canvas...");
        let pipeline = ctx.pipelines.get_or_init::<CanvasPipeline>(&ctx.wgpu_ctx);
        let mut encoder =
            ctx.wgpu_ctx
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Canvas Render Encoder"),
                });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Canvas Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &renderer.multisample_view,
                    resolve_target: Some(&renderer.render_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &renderer.depth_stencil_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            pass.set_pipeline(pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_bind_group(0, &renderer.uniforms_bind_group.bind_group, &[]);
            pass.set_bind_group(1, &self.camera.result_bind_group.bind_group, &[]);
            pass.draw(0..self.vertex_buffer.len() as u32, 0..1);
        }
        ctx.wgpu_ctx.queue.submit(Some(encoder.finish()));
    }
}
