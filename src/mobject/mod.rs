pub mod geometry;

use std::sync::{Arc, RwLock};

use glam::{ivec3, vec3, vec4, IVec3, Mat3, Vec3};
use palette::Srgba;

use crate::pipeline::{PipelineVertex, RenderPipeline};
use crate::utils::{resize_preserving_order, Id};
use crate::{WgpuBuffer, WgpuContext};

pub struct ExtractedMobject<Vertex: PipelineVertex> {
    pub id: Id,
    pub pipeline_id: std::any::TypeId,
    pub points: Arc<RwLock<Vec<Vertex>>>,
    pub(crate) buffer: WgpuBuffer<Vertex>,
}

impl<Vertex: PipelineVertex> ExtractedMobject<Vertex> {
    pub(crate) fn update_buffer(&mut self, ctx: &WgpuContext) {
        self.buffer
            .prepare_from_slice(ctx, &self.points.read().unwrap());
    }

    pub(crate) fn prepare(&mut self, ctx: &WgpuContext) {
        self.update_buffer(ctx);
    }

    pub fn render(&mut self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw(0..self.buffer.len() as u32, 0..1);
    }
}

pub trait ToMobject {
    type Pipeline: RenderPipeline + 'static;

    fn vertex(&self) -> Vec<<Self::Pipeline as RenderPipeline>::Vertex>;

    fn to_mobject(self) -> Mobject<<Self::Pipeline as RenderPipeline>::Vertex>
    where
        Self: Sized,
    {
        let points = self.vertex();
        Mobject::new::<Self::Pipeline>(points)
    }
}

#[derive(Clone)]
pub struct Mobject<Vertex: PipelineVertex> {
    id: Id,
    pipeline_id: std::any::TypeId,
    points: Arc<RwLock<Vec<Vertex>>>,
}

impl<Vertex: PipelineVertex> Mobject<Vertex> {
    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn pipeline_id(&self) -> &std::any::TypeId {
        &self.pipeline_id
    }

    fn new<Pipeline: RenderPipeline + 'static>(points: impl Into<Vec<Vertex>>) -> Self {
        Self {
            id: Id::new(),
            pipeline_id: std::any::TypeId::of::<Pipeline>(),
            points: Arc::new(RwLock::new(points.into())),
        }
    }

    pub(crate) fn extract(&self, ctx: &WgpuContext) -> ExtractedMobject<Vertex> {
        let Mobject {
            id,
            pipeline_id,
            points,
        } = self.clone();
        let buffer = WgpuBuffer::new_init(
            &ctx,
            &self.points.read().unwrap(),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );
        ExtractedMobject {
            id,
            pipeline_id,
            points,
            buffer,
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

impl<Vertex: PipelineVertex> Mobject<Vertex> {
    /// Get the bounding box of the mobject.
    /// min, mid, max
    pub fn get_bounding_box(&self) -> [Vec3; 3] {
        let points = self.points.read().unwrap();

        let min = points
            .iter()
            .map(|p| p.position())
            .reduce(|acc, e| acc.min(e))
            .unwrap();
        let max = points
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
        f: impl Fn(&mut Vec<Vertex>),
        anchor: TransformAnchor,
    ) {
        let anchor = match anchor {
            TransformAnchor::Point(x) => x,
            TransformAnchor::Edge(x) => self.get_bounding_box_point(x),
        };

        let mut points = self.points.write().unwrap();
        if anchor != Vec3::ZERO {
            points
                .iter_mut()
                .for_each(|p| p.set_position(p.position() + anchor));
        }

        f(&mut points);

        if anchor != Vec3::ZERO {
            points
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
}

impl<Vertex: PipelineVertex> Mobject<Vertex> {
    pub fn set_color(&mut self, color: Srgba) -> &mut Self {
        let color = vec4(color.red, color.green, color.blue, color.alpha);

        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    p.set_color(color);
                });
            },
            TransformAnchor::origin(),
        );
        self
    }
    pub fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    p.set_color(vec4(p.color().x, p.color().y, p.color().z, opacity));
                });
            },
            TransformAnchor::origin(),
        );
        self
    }
}

impl<Vertex: PipelineVertex> Mobject<Vertex> {
    pub fn vertex_cnt(&self) -> usize {
        self.points.read().unwrap().len()
    }

    pub fn resize_points(&mut self, len: usize) {
        let mut points = self.points.write().unwrap();
        *points = resize_preserving_order(&points, len);
    }

    pub fn aligned_with_mobject(&self, target: &Mobject<Vertex>) -> bool {
        self.vertex_cnt() == target.vertex_cnt()
    }

    pub fn align_with_mobject(&mut self, target: &mut Mobject<Vertex>) {
        let max_len = self.vertex_cnt().max(target.vertex_cnt());
        self.resize_points(max_len);
        target.resize_points(max_len);
    }

    pub fn interpolate_with_mobject(&mut self, target: &Mobject<Vertex>, t: f32) {
        let mut points = self.points.write().unwrap();
        points
            .iter_mut()
            .zip(target.points.read().unwrap().iter())
            .for_each(|(p1, p2)| {
                p1.set_position(p1.position().lerp(p2.position(), t));
                p1.set_color(p1.color().lerp(p2.color(), t));
            });
    }
}
