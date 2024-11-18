pub mod geometry;

use crate::pipeline::PipelineVertex;
use crate::{WgpuBuffer, WgpuContext};

pub struct Mobject<Vertex: PipelineVertex> {
    points: Vec<Vertex>,
    buffer: WgpuBuffer<Vertex>,
}

impl<Vertex: PipelineVertex> Mobject<Vertex> {
    pub fn from_pipeline_vertex(ctx: &WgpuContext, points: impl Into<Vec<Vertex>>) -> Self {
        let points = points.into();
        let buffer = WgpuBuffer::new_init(&ctx, &points, wgpu::BufferUsages::VERTEX);
        Self { points, buffer }
    }

    pub fn update_from_pipeline_vertex(&mut self, ctx: &WgpuContext, points: Vec<Vertex>) {
        self.points = points;
        self.buffer.prepare_from_slice(ctx, &self.points);
    }

    pub fn vertex_buffer(&self) -> &WgpuBuffer<Vertex> {
        &self.buffer
    }

    pub fn render(
        &self,
        pipeline: &Vertex::Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        depth_view: Option<&wgpu::TextureView>,
        bindgroups: &[&wgpu::BindGroup],
    ) {
        let render_pass_desc = wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: depth_view.map(|view| {
                wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        };
        let mut render_pass = encoder.begin_render_pass(&render_pass_desc);
        render_pass.set_pipeline(&pipeline);
        for (i, bindgroup) in bindgroups.iter().cloned().enumerate() {
            render_pass.set_bind_group(i as u32, bindgroup, &[]);
        }
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw(0..self.buffer.len() as u32, 0..1);
    }
}
