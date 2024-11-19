pub mod geometry;

use std::sync::{Arc, Mutex};

use glam::{ivec3, vec3, vec4, IVec3, Mat3, Vec3};

use crate::pipeline::{simple, PipelineVertex};
use crate::utils::{resize_preserving_order, Id};
use crate::{WgpuBuffer, WgpuContext};

// TODO: instead of cloning, implement a ExtractMobject

#[derive(Clone)]
pub struct Mobject<Vertex: PipelineVertex> {
    pub id: Id,
    points: Vec<Vertex>,
    buffer: Arc<Mutex<WgpuBuffer<Vertex>>>,
}

impl<Vertex: PipelineVertex> Mobject<Vertex> {
    pub fn from_pipeline_vertex(ctx: &WgpuContext, points: impl Into<Vec<Vertex>>) -> Self {
        let points = points.into();
        let buffer = WgpuBuffer::new_init(
            &ctx,
            &points,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );
        Self {
            id: Id::new(),
            points,
            buffer: Arc::new(Mutex::new(buffer)),
        }
    }

    pub fn update_from_pipeline_vertex(&mut self, ctx: &WgpuContext, points: Vec<Vertex>) {
        self.points = points;
        self.update_buffer(ctx);
    }

    pub fn update_buffer(&mut self, ctx: &WgpuContext) {
        self.buffer
            .lock()
            .unwrap()
            .prepare_from_slice(ctx, &self.points);
    }

    pub fn vertex_buffer(&self) -> &Mutex<WgpuBuffer<Vertex>> {
        &self.buffer
    }

    pub fn prepare(&mut self, ctx: &WgpuContext) {
        self.update_buffer(ctx);
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
        {
            let buffer = self.buffer.lock().unwrap();
            render_pass.set_vertex_buffer(0, buffer.slice(..));
            render_pass.draw(0..buffer.len() as u32, 0..1);
        }
    }
}

pub enum TransformAnchor {
    Point(Vec3),
    Edge(IVec3),
}

impl TransformAnchor {
    pub fn point(x: f32, y: f32, z: f32) -> Self {
        Self::Point(vec3(x, y, z))
    }

    pub fn origin() -> Self {
        Self::Point(Vec3::ZERO)
    }

    pub fn edge(x: i32, y: i32, z: i32) -> Self {
        Self::Edge(ivec3(x, y, z))
    }
}

impl Mobject<simple::Vertex> {
    /// Get the bounding box of the mobject.
    /// min, mid, max
    pub fn get_bounding_box(&self) -> [Vec3; 3] {
        let min = self
            .points
            .iter()
            .map(|p| p.position)
            .reduce(|acc, e| acc.min(e))
            .unwrap();
        let max = self
            .points
            .iter()
            .map(|p| p.position)
            .reduce(|acc, e| acc.min(e))
            .unwrap();
        let mid = (min + max) / 2.0;
        [min, mid, max]
    }

    pub fn get_bounding_box_point(&self, edge: IVec3) -> Vec3 {
        let bb = self.get_bounding_box();
        let signum = (edge.signum() + IVec3::ONE).as_uvec3();

        return vec3(
            bb[signum.x as usize].x,
            bb[signum.y as usize].y,
            bb[signum.z as usize].z,
        );
    }

    /// Apply a function to the points of the mobject about the point.
    pub fn apply_points_function(
        &mut self,
        f: impl Fn(&mut Vec<simple::Vertex>),
        anchor: TransformAnchor,
    ) {
        let anchor = match anchor {
            TransformAnchor::Point(x) => x,
            TransformAnchor::Edge(x) => self.get_bounding_box_point(x),
        };

        let mut points = self.points.clone();
        if anchor != Vec3::ZERO {
            points
                .iter_mut()
                .for_each(|p| p.set_position(p.position + anchor));
        }

        f(&mut points);
        if anchor != Vec3::ZERO {
            points
                .iter_mut()
                .for_each(|p| p.set_position(p.position - anchor));
        }
        self.points = points;
    }

    /// Shift the mobject by a given vector.
    pub fn shift(&mut self, shift: Vec3) {
        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    p.set_position(p.position + shift);
                });
            },
            TransformAnchor::origin(),
        );
    }

    /// Scale the mobject by a given vector.
    pub fn scale(&mut self, scale: Vec3, anchor: TransformAnchor) {
        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    p.set_position(p.position * scale);
                });
            },
            anchor,
        );
    }

    /// Rotate the mobject by a given angle about a given axis.
    pub fn rotate(&mut self, angle: f32, axis: Vec3, anchor: TransformAnchor) {
        let axis = axis.normalize();
        let rotation = Mat3::from_axis_angle(axis, angle);

        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    p.set_position(rotation * p.position);
                });
            },
            anchor,
        );
    }
}

impl Mobject<simple::Vertex> {
    pub fn set_opacity(&mut self, opacity: f32) {
        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    p.set_color(vec4(p.color.x, p.color.y, p.color.z, opacity));
                });
            },
            TransformAnchor::origin(),
        );
    }
}

impl Mobject<simple::Vertex> {
    pub fn resize_points(&mut self, len: usize) {
        self.points = resize_preserving_order(&self.points, len);
    }

    pub fn aligned_with_mobject(&self, target: &Mobject<simple::Vertex>) -> bool {
        self.points.len() == target.points.len()
    }

    pub fn align_with_mobject(&mut self, target: &mut Mobject<simple::Vertex>) {
        let max_len = self.points.len().max(target.points.len());
        self.resize_points(max_len);
        target.resize_points(max_len);
    }

    pub fn interpolate_with_mobject(&mut self, target: &Mobject<simple::Vertex>, t: f32) {
        let points = self
            .points
            .iter()
            .zip(target.points.iter())
            .map(|(p1, p2)| {
                let position = p1.position.lerp(p2.position, t);
                let color = p1.color.lerp(p2.color, t);
                simple::Vertex::new(position, color)
            })
            .collect();
        self.points = points;
    }
}
