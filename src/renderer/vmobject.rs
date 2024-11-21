use palette::{rgb, Srgba};

use crate::{mobject::ExtractedMobject, pipeline::simple, RanimContext};

use super::{Renderer, RendererVertex};

use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct VMobjectVertex {
    pub position: Vec3,
    pub(crate) _padding: f32,
    pub color: Vec4,
}

impl Default for VMobjectVertex {
    fn default() -> Self {
        Self::new(Vec3::ZERO, Vec4::ZERO)
    }
}

impl RendererVertex for VMobjectVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem::size_of;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }

    fn position(&self) -> Vec3 {
        self.position
    }

    fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    fn color(&self) -> Vec4 {
        self.color
    }

    fn set_color(&mut self, color: Vec4) {
        self.color = color;
    }

    fn interpolate(&self, other: &Self, t: f32) -> Self {
        Self {
            position: self.position.lerp(other.position, t),
            color: self.color.lerp(other.color, t),
            ..*self
        }
    }
}

impl VMobjectVertex {
    pub fn new(position: Vec3, color: Vec4) -> Self {
        Self {
            position,
            _padding: 0.0,
            color,
        }
    }
}

pub struct VMobjectRenderer;

impl Renderer for VMobjectRenderer {
    type Vertex = VMobjectVertex;

    fn begin_pass<'a>(
        encoder: &'a mut wgpu::CommandEncoder,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
    ) -> wgpu::RenderPass<'a> {
        let bg = Srgba::from_u32::<rgb::channels::Rgba>(0x333333FF).into_linear();
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &multisample_view,
                resolve_target: Some(&target_view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: bg.red,
                        g: bg.green,
                        b: bg.blue,
                        a: bg.alpha,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        })
    }

    fn render<'a>(
        ctx: &mut RanimContext,
        pass: &mut wgpu::RenderPass<'a>,
        mobjects: &mut Vec<&mut ExtractedMobject<Self::Vertex>>,
    ) {
        pass.set_pipeline(ctx.get_or_init_pipeline::<simple::Pipeline>());
        for mobject in mobjects {
            mobject.render(pass);
        }
    }
}
