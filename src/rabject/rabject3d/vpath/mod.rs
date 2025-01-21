use bevy_color::{Alpha, LinearRgba, Srgba};
use glam::Vec3;
use pipeline::VPathFillVertex;
use primitive::{ExtractedVPath, VPathPrimitive};

use crate::{prelude::Interpolatable, utils::rotation_between_vectors};

use super::Rabject;

pub mod blueprint;
pub mod pipeline;
pub mod primitive;

#[derive(Default, Debug, Clone, Copy)]
pub struct VPathPoint {
    pub position: Vec3,
    pub prev_handle: Option<Vec3>,
    pub next_handle: Option<Vec3>,
    pub stroke_color: LinearRgba,
    pub stroke_width: f32,
    pub fill_color: LinearRgba,
}

impl Interpolatable for VPathPoint {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        let prev_handle = self.prev_handle.and_then(|h| {
            target
                .prev_handle
                .and_then(|h_target| Some(h.lerp(h_target, t)))
        });
        let next_handle = self.next_handle.and_then(|h| {
            target
                .next_handle
                .and_then(|h_target| Some(h.lerp(h_target, t)))
        });
        Self {
            position: self.position.lerp(target.position, t),
            prev_handle,
            next_handle,
            stroke_color: self.stroke_color.lerp(&target.stroke_color, t),
            stroke_width: self.stroke_width.lerp(&target.stroke_width, t),
            fill_color: self.fill_color.lerp(&target.fill_color, t),
        }
    }
}

impl VPathPoint {
    pub fn new(pos: Vec3, prev_handle: Option<Vec3>, next_handle: Option<Vec3>) -> Self {
        Self {
            position: pos,
            prev_handle,
            next_handle,
            stroke_color: Srgba::RED.with_alpha(0.5).into(),
            stroke_width: 10.0,
            fill_color: Srgba::BLUE.with_alpha(0.2).into(),
        }
    }
    pub fn set_stroke_color(&mut self, color: LinearRgba) {
        self.stroke_color = color;
    }
    pub fn set_stroke_width(&mut self, width: f32) {
        self.stroke_width = width;
    }
    pub fn set_fill_color(&mut self, color: LinearRgba) {
        self.fill_color = color;
    }
}

#[derive(Default, Debug, Clone)]
pub struct VPath {
    pub points: Vec<VPathPoint>,
    pub paint_order: usvg::PaintOrder,
}

impl Rabject for VPath {
    type ExtractData = ExtractedVPath;
    type RenderResource = VPathPrimitive;

    fn extract(&self) -> Self::ExtractData {
        let joint_angles = self.get_joint_angles();
        let points = self
            .points
            .iter()
            .zip(joint_angles)
            .map(|(p, joint_angle)| primitive::VPathPoint {
                position: p.position.extend(1.0),
                prev_handle: p.prev_handle.unwrap_or(p.position).extend(1.0),
                next_handle: p.next_handle.unwrap_or(p.position).extend(1.0),
                fill_color: p.fill_color.into(),
                stroke_color: p.stroke_color.into(),
                stroke_width: p.stroke_width,
                joint_angle,
                _padding: [0.0; 2],
            })
            .collect::<Vec<_>>();

        ExtractedVPath {
            points,
            unit_normal: self.get_unit_normal(),
            fill_triangles: self.parse_fill(),
            render_order: self.paint_order,
        }
    }
}

impl VPath {
    pub fn is_closed(&self) -> bool {
        if self.points.is_empty() {
            return false;
        }

        (self.points.first().unwrap().position - self.points.last().unwrap().position).length()
            < f32::EPSILON
    }
    pub fn get_unit_normal(&self) -> Vec3 {
        if !self.is_closed() || self.points.len() < 5 {
            return Vec3::Z;
        }

        let normal = self
            .points
            .iter()
            .map(|p| p.position)
            .skip(1)
            .fold((Vec3::ZERO, self.points[0].position), |(acc, pre), e| {
                (acc + pre.cross(e), e)
            })
            .0;
        if normal.length() < f32::EPSILON {
            return Vec3::Z;
        }
        normal.normalize()
    }
    pub fn get_joint_angles(&self) -> Vec<f32> {
        assert!(self.points.len() >= 2);
        let joint_angles = self
            .points
            .iter()
            .map(|point| {
                let (Some(hpre), Some(hnext)) = (point.prev_handle, point.next_handle) else {
                    return 0.0;
                };
                let anchor = point.position;
                // trace!(
                //     "[{anchor_index}]: anchor: {:?}, hpre: {:?}, hnext: {:?}",
                //     anchor,
                //     hpre,
                //     hnext
                // );

                let v_in = (anchor - hpre).normalize();
                let v_out = (hnext - anchor).normalize();
                // trace!("[{anchor_index}]: v_in: {:?}, v_out: {:?}", v_in, v_out);

                let unit_normal = self.get_unit_normal();
                // trace!("[{anchor_index}]: unit_normal: {:?}", unit_normal);

                let mat = rotation_between_vectors(Vec3::Z, unit_normal);
                // trace!("[{anchor_index}]: mat: {:?}", mat);

                let v_in = mat * v_in;
                let v_out = mat * v_out;
                // trace!("[{anchor_index}]: v_in: {:?}, v_out: {:?}", v_in, v_out);

                let v_in_angle = v_in.y.atan2(v_in.x);
                let v_out_angle = v_out.y.atan2(v_out.x);
                let angle = v_out_angle - v_in_angle;
                // println!("[{anchor_index}]: angle: {:?}", angle);
                if angle > std::f32::consts::PI {
                    angle - std::f32::consts::TAU
                } else if angle < -std::f32::consts::PI {
                    angle + std::f32::consts::TAU
                } else {
                    angle
                }
            })
            .collect::<Vec<_>>();
        joint_angles
    }

    // TODO: do this in compute shader
    pub fn parse_fill(&self) -> Vec<VPathFillVertex> {
        let points = &self.points;
        if points.is_empty() || !self.is_closed() {
            return vec![VPathFillVertex::default(); 3];
        }

        let mut vertices = Vec::with_capacity(points.len() * 3); // not acurate
        let base_point = points.first().unwrap();
        // let unit_normal = self.get_unit_normal();
        points
            .iter()
            .zip(points.iter().skip(1))
            .for_each(|(p0, p1)| {
                let v0 = p0.position - base_point.position;
                let v1 = p1.position - base_point.position;
                // let mat = Mat3::from_cols(unit_normal, v1, v2);
                // let face = mat.determinant();
                let h0 = p0.next_handle.unwrap_or(p0.position);
                let h0 = VPathPoint {
                    position: h0,
                    ..p0.lerp(p1, 0.33)
                };
                let h1 = p1.prev_handle.unwrap_or(p1.position);
                let h1 = VPathPoint {
                    position: h1,
                    ..p0.lerp(p1, 0.66)
                };

                let normal = v0.cross(v1).normalize();
                if normal == Vec3::ZERO {
                    return;
                }
                vertices.extend_from_slice(&[(*base_point, 0), (*p0, 0), (*p1, 0)]);
                vertices.extend_from_slice(&[(*p0, 1), (h0, 1), (h1, 1)]);
                vertices.extend_from_slice(&[(*p0, 2), (h1, 2), (*p1, 2)]);
            });
        vertices
            .into_iter()
            .map(|(v, fill_type)| VPathFillVertex {
                pos: v.position,
                fill_type,
                fill_color: v.fill_color,
            })
            .collect()
    }
}

impl VPath {
    pub fn line(start: Vec3, end: Vec3) -> Self {
        let mid = (start + end) / 2.0;
        Self {
            points: vec![
                VPathPoint::new(start, None, Some(mid)),
                VPathPoint::new(end, Some(mid), None),
            ],
            ..Default::default()
        }
    }

    pub fn quad(start: Vec3, control: Vec3, end: Vec3) -> Self {
        Self {
            points: vec![
                VPathPoint::new(start, None, Some(control)),
                VPathPoint::new(end, Some(control), None),
            ],
            ..Default::default()
        }
    }

    pub fn cubic(start: Vec3, control1: Vec3, control2: Vec3, end: Vec3) -> Self {
        Self {
            points: vec![
                VPathPoint::new(start, None, Some(control1)),
                VPathPoint::new(end, Some(control2), None),
            ],
            ..Default::default()
        }
    }

    pub fn set_stroke_color(&mut self, color: LinearRgba) {
        self.points
            .iter_mut()
            .for_each(|p| p.set_stroke_color(color));
    }
    pub fn set_stroke_width(&mut self, width: f32) {
        self.points
            .iter_mut()
            .for_each(|p| p.set_stroke_width(width));
    }
    pub fn set_fill_color(&mut self, color: LinearRgba) {
        self.points.iter_mut().for_each(|p| p.set_fill_color(color));
    }
}
