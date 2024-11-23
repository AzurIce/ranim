pub mod geometry;

use std::sync::{Arc, RwLock};

use glam::{ivec3, vec2, vec3, vec4, IVec3, Mat3, Vec3};
use palette::Srgba;

use crate::renderer::{Renderer, RendererVertex};
use crate::utils::{extend_with_last, resize_preserving_order, Id};
use crate::{WgpuBuffer, WgpuContext};

pub trait ToMobject {
    type Renderer: Renderer + 'static;

    fn to_mobject(self) -> Mobject<Self::Renderer>
    where
        Self: Sized;
}

pub struct ExtractedMobject<Vertex: RendererVertex> {
    pub id: Id,
    pub renderer_id: std::any::TypeId,
    pub points: Arc<RwLock<Vec<Vertex>>>,
    pub(crate) buffer: WgpuBuffer<Vertex>,
}

pub struct Mobject<R: Renderer + 'static> {
    id: Id,
    points: Arc<RwLock<Vec<R::Vertex>>>,
}

impl<R: Renderer> Clone for Mobject<R> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            points: self.points.clone(),
        }
    }
}

impl<R: Renderer> Mobject<R> {
    pub(crate) fn into_points(self) -> Vec<R::Vertex> {
        self.points.read().unwrap().clone()
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub(crate) fn new(points: impl Into<Vec<R::Vertex>>) -> Self {
        Self {
            id: Id::new(),
            points: Arc::new(RwLock::new(points.into())),
        }
    }

    pub(crate) fn extract(&self, ctx: &WgpuContext) -> ExtractedMobject<R::Vertex> {
        let Mobject { id, points } = self.clone();
        let buffer = WgpuBuffer::new_init(
            &ctx,
            &self.points.read().unwrap(),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );
        ExtractedMobject {
            id,
            renderer_id: std::any::TypeId::of::<R>(),
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

impl<R: Renderer> Mobject<R> {
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
        f: impl Fn(&mut Vec<R::Vertex>),
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

    pub fn get_start_position(&self) -> Option<Vec3> {
        self.points.read().unwrap().first().map(|p| p.position())
    }

    pub fn get_end_position(&self) -> Option<Vec3> {
        self.points.read().unwrap().last().map(|p| p.position())
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

impl<R: Renderer> Mobject<R> {
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

impl<R: Renderer> Mobject<R> {
    pub fn vertex_cnt(&self) -> usize {
        self.points.read().unwrap().len()
    }

    pub fn resize_points(&mut self, len: usize) {
        let mut points = self.points.write().unwrap();
        if points.len() < len {
            extend_with_last(&mut points, len);
        } else {
            *points = resize_preserving_order(&points, len);
        }
    }

    pub fn aligned_with_mobject(&self, target: &Mobject<R>) -> bool {
        self.vertex_cnt() == target.vertex_cnt()
    }

    pub fn align_with_mobject(&mut self, target: &mut Mobject<R>) {
        let max_len = self.vertex_cnt().max(target.vertex_cnt());
        self.resize_points(max_len);
        target.resize_points(max_len);
    }

    pub fn interpolate_with_mobject(&mut self, target: &Mobject<R>, t: f32) {
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
