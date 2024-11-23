use glam::Vec4;

use crate::{RanimContext, WgpuBuffer};

use super::Rabject;

#[derive(Clone)]
pub struct VMobjectPoint {
    pub pos: Vec4,
    pub stroke_color: Vec4,
    pub stroke_width: f32,
    pub fill_color: Vec4,
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct VMobjectFillVertex {
    pub pos: Vec4,
    pub fill_color: Vec4,
    pub unit_normal: Vec4,
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct VMobjectStrokeVertex {
    pub pos: Vec4,
    pub stroke_color: Vec4,
}

impl From<VMobjectPoint> for VMobjectFillVertex {
    fn from(point: VMobjectPoint) -> Self {
        Self {
            pos: point.pos,
            fill_color: point.fill_color,
            unit_normal: Vec4::ZERO,
        }
    }
}

pub struct VMobject {
    points: Vec<VMobjectPoint>,
}

impl VMobject {
    pub fn from_points(points: Vec<VMobjectPoint>) -> Self {
        Self { points }
    }

    fn parse_stroke(&self) -> Vec<VMobjectStrokeVertex> {
        // TODO: implement bezier
        return vec![];
    }

    fn parse_fill(&self) -> Vec<VMobjectFillVertex> {
        if self.points.is_empty() {
            return vec![];
        }

        let mut vertices = Vec::with_capacity(self.points.len() * 3); // not acurate
        let base_point = self.points.first().unwrap();
        self.points
            .iter()
            .cloned()
            .zip(
                self.points
                    .iter()
                    .skip(1)
                    .cloned()
                    .zip(self.points.iter().skip(2).cloned()),
            )
            .map(|(p1, (p2, p3))| {
                vertices.extend_from_slice(&[base_point.clone(), p1, p3])
                // vertices.extend_from_slice(&[p1, p2, p3]);
            });
        vertices.into_iter().map(|v| v.into()).collect()
    }
}

pub struct VMObjectRenderResource {
    fill_vertex_buffer: WgpuBuffer<VMobjectFillVertex>,
    stroke_vertex_buffer: WgpuBuffer<VMobjectStrokeVertex>,
}

impl Rabject for VMobject {
    type RenderResource = VMObjectRenderResource;

    fn init_render_resource(ctx: &mut RanimContext, rabject: &Self) -> Self::RenderResource {
        Self::RenderResource {
            fill_vertex_buffer: WgpuBuffer::new_init(
                &ctx.wgpu_ctx,
                &rabject.parse_fill(),
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            ),
            stroke_vertex_buffer: WgpuBuffer::new_init(
                &ctx.wgpu_ctx,
                &rabject.parse_stroke(),
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            ),
        }
    }

    fn update_render_resource(
        ctx: &mut crate::RanimContext,
        rabject: &Self,
        render_resource: &mut Self::RenderResource,
    ) {
        render_resource
            .fill_vertex_buffer
            .prepare_from_slice(&ctx.wgpu_ctx, &rabject.parse_fill());
        render_resource
            .stroke_vertex_buffer
            .prepare_from_slice(&ctx.wgpu_ctx, &rabject.parse_stroke());
    }

    fn render(ctx: &mut crate::RanimContext, render_resource: &Self::RenderResource) {}
}
