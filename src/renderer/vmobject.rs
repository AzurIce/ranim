use bezier_rs::Bezier;
use flo_curves::bezier::path::BezierPathBuilder;
use palette::{rgb, Srgba};

use crate::{mobject::ExtractedMobject, pipeline::{simple, PipelineVertex}, RanimContext};

use super::Renderer;

use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
/// VMobjectVertex is the data of vectorized path,
/// they will be used to generate the actual vertices on [`VMobjectRenderer::prepare`]
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

impl PipelineVertex for VMobjectVertex {
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

    fn prepare(ctx: &mut RanimContext, mobjects: &mut Vec<&mut ExtractedMobject<Self::Vertex>>) {
        for mobject in mobjects {
            let points = mobject.points.read().unwrap();

            let mut path = BezierPathBuilder::start(points[0].position());
            for [cp, p] in points[1..].chunks(2) {
                path.curve_to((cp.position(), cp.position()), p.position());
            }

            let beziers = points
                .iter()
                .step_by(2)
                .zip(points.iter().skip(1).step_by(2))
                .zip(points.iter().skip(2).step_by(2))
                .map(|((&p1, &p2), &p3)| {
                    let [p1, p2, p3] = [p1, p2, p3].map(|p| p.position().as_dvec2());
                    Bezier::from_quadratic_dvec2(p1.as_dvec2(), p2.as_dvec2(), p3.as_dvec2())
                })
                .collect::<Vec<_>>();

            mobject
                .buffer
                .prepare_from_slice(&ctx.wgpu_ctx, &mobject.points.read().unwrap());
        }
    }

    fn render<'a>(
        ctx: &mut RanimContext,
        pass: &mut wgpu::RenderPass<'a>,
        mobjects: &mut Vec<&mut ExtractedMobject<Self::Vertex>>,
    ) {
        pass.set_pipeline(ctx.get_or_init_pipeline::<simple::Pipeline>());
        for mobject in mobjects {
            pass.set_vertex_buffer(0, mobject.buffer.slice(..));
            pass.draw(0..mobject.buffer.len() as u32, 0..1);
        }
    }
}
