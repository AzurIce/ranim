use glam::{ivec3, vec2, vec3, vec4, IVec3, Mat3, Vec3, Vec4};
use itertools::Itertools;
use log::trace;
use palette::{rgb, Srgba};

use crate::{
    pipeline::{
        vmobject_fill::{self, VMobjectFillVertex},
        PipelineVertex,
    },
    utils::{extend_with_last, resize_preserving_order},
    RanimContext, WgpuBuffer,
};

use super::{ExtractedRabjectWithId, Interpolatable, Rabject, RabjectWithId};

#[derive(Clone, Default, Debug)]
pub struct VMobjectPoint {
    pub pos: Vec3,
    pub fill_color: Vec4,
    pub stroke_color: Vec4,
    pub stroke_width: f32,
}

impl Interpolatable for VMobjectPoint {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Self {
            pos: self.pos.lerp(target.pos, t),
            stroke_color: self.stroke_color.lerp(target.stroke_color, t),
            stroke_width: self.stroke_width.lerp(&target.stroke_width, t),
            fill_color: self.fill_color.lerp(target.fill_color, t),
        }
    }
}

impl VMobjectPoint {
    pub fn position(&self) -> Vec3 {
        vec3(self.pos.x, self.pos.y, self.pos.z)
    }
    pub fn set_position(&mut self, pos: Vec3) {
        self.pos = vec3(pos.x, pos.y, pos.z);
    }
    pub fn stroke_color(&self) -> Vec4 {
        self.stroke_color
    }
    pub fn set_stroke_color(&mut self, color: Vec4) {
        self.fill_color = color;
    }
    pub fn fill_color(&self) -> Vec4 {
        self.fill_color
    }
    pub fn set_fill_color(&mut self, color: Vec4) {
        self.fill_color = color;
    }
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct VMobjectStrokeVertex {
    pub pos: Vec4,
    pub stroke_color: Vec4,
}

impl From<VMobjectPoint> for VMobjectFillVertex {
    fn from(point: VMobjectPoint) -> Self {
        // trace!("[VMobject] VMobjectFillVertex: {:?}", point);
        Self {
            pos: point.pos.extend(1.0),
            fill_color: point.fill_color,
            unit_normal: Vec4::ZERO,
        }
    }
}

#[derive(Clone, Default)]
pub struct VMobject {
    points: Vec<VMobjectPoint>, // anchor-handle-anchor-handle-...-anchor
}

impl VMobject {
    /// Create a VMobject from a list of corner points.
    pub fn from_corner_points(mut points: Vec<Vec3>) -> Self {
        if points.is_empty() {
            return Self::from_points(vec![]);
        }
        // Close the polygon
        points.push(points[0]);

        let anchors = points;
        let handles = anchors
            .windows(2)
            .map(|window| 0.5 * (window[0] + window[1]))
            .collect::<Vec<_>>();

        // Interleave anchors and handles
        let points = anchors
            .into_iter()
            .interleave(handles.into_iter())
            .collect();

        Self::from_points(points)
    }

    pub fn from_points(points: Vec<Vec3>) -> Self {
        trace!("[VMobject] from_points");
        assert!(points.len() % 2 == 1); // must be odd number of points

        let points = points
            .into_iter()
            .map(|p| VMobjectPoint {
                pos: p,
                stroke_color: vec4(1.0, 0.0, 0.0, 1.0),
                stroke_width: 1.0,
                fill_color: vec4(0.0, 0.0, 1.0, 0.5),
            })
            .collect();

        Self { points }
    }

    fn parse_stroke(&self) -> Vec<VMobjectStrokeVertex> {
        if self.points.is_empty() {
            return vec![];
        }

        // TODO: implement bezier
        return vec![];
    }

    pub fn parse_fill(&self) -> Vec<VMobjectFillVertex> {
        if self.points.is_empty() {
            return vec![];
        }

        trace!(
            "[VMobject] parse_fill: {:?}",
            self.points.first().unwrap().fill_color
        );
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
            .for_each(|(p1, (p2, p3))| {
                vertices.extend_from_slice(&[base_point.clone(), p1.clone(), p3.clone()]);
                vertices.extend_from_slice(&[p1.clone(), p2.clone(), p3.clone()]);
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
        rabject: &RabjectWithId<Self>,
        render_resource: &mut Self::RenderResource,
    ) {
        let fill_vertices = rabject.parse_fill();
        trace!("[VMobject] update_render_resource: fill_vertices");
        render_resource
            .fill_vertex_buffer
            .prepare_from_slice(&ctx.wgpu_ctx, &fill_vertices);
        let stroke_vertices = rabject.parse_stroke();
        render_resource
            .stroke_vertex_buffer
            .prepare_from_slice(&ctx.wgpu_ctx, &stroke_vertices);
    }

    fn begin_render_pass<'a>(
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
        ctx: &mut crate::RanimContext,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_resource: &Self::RenderResource,
    ) {
        let pipeline_vmobject_fill = ctx.get_or_init_pipeline::<vmobject_fill::Pipeline>();
        render_pass.set_pipeline(&pipeline_vmobject_fill);
        render_pass.set_vertex_buffer(0, render_resource.fill_vertex_buffer.slice(..));
        render_pass.draw(0..render_resource.fill_vertex_buffer.len() as u32, 0..1);
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

impl VMobject {
    /// Get the bounding box of the mobject.
    /// min, mid, max
    pub fn get_bounding_box(&self) -> [Vec3; 3] {
        let min = self
            .points
            .iter()
            .map(|p| p.position())
            .reduce(|acc, e| acc.min(e))
            .unwrap();
        let max = self
            .points
            .iter()
            .map(|p| p.position())
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
        f: impl Fn(&mut Vec<VMobjectPoint>),
        anchor: TransformAnchor,
    ) {
        let anchor = match anchor {
            TransformAnchor::Point(x) => x,
            TransformAnchor::Edge(x) => self.get_bounding_box_point(x),
        };

        if anchor != Vec3::ZERO {
            self.points
                .iter_mut()
                .for_each(|p| p.set_position(p.position() + anchor));
        }

        f(&mut self.points);

        if anchor != Vec3::ZERO {
            self.points
                .iter_mut()
                .for_each(|p| p.set_position(p.position() - anchor));
        }
    }

    /// Shift the mobject by a given vector.
    pub fn shift(&mut self, shift: Vec3) -> &mut Self {
        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    p.set_position(p.position() + shift);
                });
            },
            TransformAnchor::origin(),
        );
        self
    }

    /// Scale the mobject by a given vector.
    pub fn scale(&mut self, scale: Vec3, anchor: TransformAnchor) -> &mut Self {
        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    p.set_position(p.position() * scale);
                });
            },
            anchor,
        );
        self
    }

    /// Rotate the mobject by a given angle about a given axis.
    pub fn rotate(&mut self, angle: f32, axis: Vec3, anchor: TransformAnchor) -> &mut Self {
        let axis = axis.normalize();
        let rotation = Mat3::from_axis_angle(axis, angle);

        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    p.set_position(rotation * p.position());
                });
            },
            anchor,
        );
        self
    }

    pub fn get_start_position(&self) -> Option<Vec3> {
        self.points.first().map(|p| p.position())
    }

    pub fn get_end_position(&self) -> Option<Vec3> {
        self.points.last().map(|p| p.position())
    }

    pub fn put_start_and_end_on(&mut self, start: Vec3, end: Vec3) -> &mut Self {
        let (cur_start, cur_end) = (
            self.get_start_position().unwrap_or_default(),
            self.get_end_position().unwrap_or_default(),
        );
        let cur_v = cur_end - cur_start;
        if cur_v.length_squared() <= f32::EPSILON {
            return self;
        }

        let v = end - start;
        self.scale(
            Vec3::splat(v.length() / cur_v.length()),
            TransformAnchor::Point(cur_start),
        );
        let angle = cur_v.y.atan2(-cur_v.x) - v.y.atan2(-v.x) + std::f32::consts::PI / 2.0;
        self.rotate(angle, Vec3::Z, TransformAnchor::origin());
        let cur_xy = vec2(cur_v.x, cur_v.y);
        let cur_xy = cur_xy * cur_xy.abs().normalize();

        let xy = vec2(v.x, v.y);
        let xy = xy * xy.abs().normalize();
        let angle = cur_v.z.atan2(-cur_xy.length()) - v.z.atan2(-xy.length());
        self.rotate(angle, vec3(-v.y, v.x, 0.0), TransformAnchor::origin());
        self.shift(start - self.get_start_position().unwrap());

        self
    }
}

impl VMobject {
    pub fn set_stroke_color(&mut self, color: Srgba) -> &mut Self {
        let color = vec4(color.red, color.green, color.blue, color.alpha);

        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    p.set_stroke_color(color);
                });
            },
            TransformAnchor::origin(),
        );
        self
    }
    pub fn set_fill_color(&mut self, color: Srgba) -> &mut Self {
        let color = vec4(color.red, color.green, color.blue, color.alpha);

        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    p.set_fill_color(color);
                });
            },
            TransformAnchor::origin(),
        );
        self
    }
    pub fn set_color(&mut self, color: Srgba) -> &mut Self {
        trace!("[VMobject] set_color: {:?}", color);
        self.set_fill_color(color).set_stroke_color(color);
        self
    }
    pub fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    p.set_fill_color(vec4(
                        p.fill_color().x,
                        p.fill_color().y,
                        p.fill_color().z,
                        opacity,
                    ));
                    p.set_stroke_color(vec4(
                        p.stroke_color().x,
                        p.stroke_color().y,
                        p.stroke_color().z,
                        opacity,
                    ));
                });
            },
            TransformAnchor::origin(),
        );
        self
    }
}

impl VMobject {
    pub fn resize_points(&mut self, len: usize) {
        // if self.points.len() < len {
        //     extend_with_last(&mut self.points, len);
        // } else {
        self.points = resize_preserving_order(&self.points, len);
        // }
    }

    pub fn aligned_with_rabject(&self, target: &Self) -> bool {
        self.points.len() == target.points.len()
    }

    pub fn align_with_rabject(&mut self, target: &mut Self) {
        let max_len = self.points.len().max(target.points.len());
        self.resize_points(max_len);
        target.resize_points(max_len);
    }

    pub fn interpolate_with_rabject(&mut self, target: &Self, t: f32) {
        self.points
            .iter_mut()
            .zip(target.points.iter())
            .for_each(|(p1, p2)| {
                *p1 = p1.lerp(p2, t);
            });
    }
}

impl Interpolatable for VMobject {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        let mut new = self.clone();
        new.points = self
            .points
            .iter()
            .zip(target.points.iter())
            .map(|(p1, p2)| p1.lerp(p2, t))
            .collect();
        new
    }
}
