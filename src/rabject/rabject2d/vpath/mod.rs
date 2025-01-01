use std::cmp::Ordering;
use std::fmt::Debug;

use bevy_color::{Alpha, LinearRgba, Srgba};
use glam::{vec2, vec3, IVec3, Mat3, Vec3};
use itertools::Itertools;
use log::trace;
use pipeline::VPathFillVertex;
use primitive::{ExtractedVPath, VPathPrimitive};

use crate::prelude::{Alignable, Opacity};
use crate::utils::bezier::trim_cubic_bezier;
use crate::{prelude::Interpolatable, utils::rotation_between_vectors};

use crate::rabject::{Rabject, TransformAnchor};

pub mod blueprint;
pub mod pipeline;
pub mod primitive;

#[derive(Default, Debug, Clone, Copy, PartialEq)]
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
    pub fn new(position: Vec3, prev_handle: Option<Vec3>, next_handle: Option<Vec3>) -> Self {
        Self {
            position,
            prev_handle,
            next_handle,
            stroke_color: Srgba::RED.with_alpha(0.5).into(),
            stroke_width: 10.0,
            fill_color: Srgba::BLUE.with_alpha(0.2).into(),
        }
    }
    pub fn position(&self) -> Vec3 {
        self.position
    }
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }
    pub fn stroke_color(&self) -> LinearRgba {
        self.stroke_color
    }
    pub fn set_stroke_color(&mut self, color: impl Into<LinearRgba>) {
        self.stroke_color = color.into();
    }
    pub fn fill_color(&self) -> LinearRgba {
        self.fill_color
    }
    pub fn set_fill_color(&mut self, color: impl Into<LinearRgba>) {
        // trace!("point set_fill_color: {:?}", color);
        self.fill_color = color.into();
    }
    pub fn stroke_width(&self) -> f32 {
        self.stroke_width
    }
    pub fn set_stroke_width(&mut self, width: f32) {
        self.stroke_width = width;
    }
    pub fn set_opacity(&mut self, opacity: f32) {
        self.fill_color = self.fill_color.with_alpha(opacity);
        self.stroke_color = self.stroke_color.with_alpha(opacity);
    }
}

#[derive(Default, Debug, Clone)]
pub struct VPath {
    pub points: Vec<VPathPoint>,
    pub paint_order: usvg::PaintOrder,
}

impl VPath {
    pub fn points(&self) -> &[VPathPoint] {
        &self.points
    }
}

impl Rabject for VPath {
    type RenderData = ExtractedVPath;
    type RenderResource = VPathPrimitive;

    fn extract(&self) -> Self::RenderData {
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

    pub fn set_color(&mut self, color: impl Into<LinearRgba> + Debug + Copy) -> &mut Self {
        self.set_stroke_color(color);
        self.set_fill_color(color);
        self
    }
    pub fn set_stroke_color(&mut self, color: impl Into<LinearRgba> + Debug + Copy) -> &mut Self {
        self.points
            .iter_mut()
            .for_each(|p| p.set_stroke_color(color));
        self
    }
    pub fn set_stroke_width(&mut self, width: f32) -> &mut Self {
        self.points
            .iter_mut()
            .for_each(|p| p.set_stroke_width(width));
        self
    }
    pub fn set_fill_color(&mut self, color: impl Into<LinearRgba> + Debug + Copy) -> &mut Self {
        self.points.iter_mut().for_each(|p| p.set_fill_color(color));
        self
    }
}

impl VPath {
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

        vec3(
            bb[signum.x as usize].x,
            bb[signum.y as usize].y,
            bb[signum.z as usize].z,
        )
    }
    /// Apply a function to the points of the mobject about the point.
    pub fn apply_points_function(
        &mut self,
        f: impl Fn(&mut Vec<VPathPoint>),
        anchor: TransformAnchor,
    ) {
        let anchor = match anchor {
            TransformAnchor::Point(x) => x,
            TransformAnchor::Edge(x) => self.get_bounding_box_point(x),
        };

        if anchor != Vec3::ZERO {
            self.points.iter_mut().for_each(|p| {
                p.position += anchor;
                p.prev_handle = p.prev_handle.map(|h| h + anchor);
                p.next_handle = p.next_handle.map(|h| h + anchor);
            });
        }

        f(&mut self.points);

        if anchor != Vec3::ZERO {
            self.points.iter_mut().for_each(|p| {
                p.position -= anchor;
                p.prev_handle = p.prev_handle.map(|h| h - anchor);
                p.next_handle = p.next_handle.map(|h| h - anchor);
            });
        }
    }

    /// Shift the mobject by a given vector.
    pub fn shift(&mut self, shift: Vec3) -> &mut Self {
        self.points.iter_mut().for_each(|p| {
            p.position += shift;
            p.prev_handle = p.prev_handle.map(|h| h + shift);
            p.next_handle = p.next_handle.map(|h| h + shift);
        });
        self
    }

    /// Scale the mobject by a given vector.
    pub fn scale(&mut self, scale: Vec3, anchor: TransformAnchor) -> &mut Self {
        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    p.position *= scale;
                    p.prev_handle = p.prev_handle.map(|h| h * scale);
                    p.next_handle = p.next_handle.map(|h| h * scale);
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
                    p.position = rotation * p.position;
                    p.prev_handle = p.prev_handle.map(|h| rotation * h);
                    p.next_handle = p.next_handle.map(|h| rotation * h);
                });
            },
            anchor,
        );
        self
    }
    pub fn put_start_and_end_on(&mut self, start: Vec3, end: Vec3) -> &mut Self {
        let (cur_start, cur_end) = (
            self.points.first().map(|p| p.position).unwrap_or_default(),
            self.points.last().map(|p| p.position).unwrap_or_default(),
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
        self.shift(start - self.points.first().unwrap().position);

        self
    }
}

impl VPath {
    pub fn align_to(&mut self, target: &Self) -> &mut Self {
        if self.points.len() >= target.points.len() {
            return self;
        }

        trace!(
            "[VPath] {} align to {}",
            self.points.len(),
            target.points.len()
        );

        let beziers = self
            .points
            .iter()
            .zip(self.points.iter().skip(1))
            .collect::<Vec<_>>();

        let mut lens = beziers
            .iter()
            .map(|(p0, p1)| (p1.position - p0.position).length())
            .collect::<Vec<_>>();
        // println!("{:?}", lens);

        let n = target.points.len() - self.points.len();
        let mut ipc = vec![0; beziers.len()];
        for _ in 0..n {
            let i = lens
                .iter()
                .position_max_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
                .unwrap();
            ipc[i] += 1;
            lens[i] *= ipc[i] as f32 / (ipc[i] + 1) as f32;
        }

        // println!("{:?}", lens);
        // println!("{:?}", ipc);

        let mut new_points = vec![self.points[0]];
        for (bezier, ipc) in beziers.iter().zip(ipc.into_iter()) {
            trace!("bezier: {:?}, ipc: {}", bezier, ipc);
            let alphas = (0..ipc + 2)
                .map(|i| i as f32 / (ipc + 1) as f32)
                .collect::<Vec<_>>();
            trace!("alphas: {:?}", alphas);

            for (a, b) in alphas.iter().zip(alphas.iter().skip(1)) {
                // let mut point_a = bezier.0.lerp(bezier.1, *a);
                let mut point_b = bezier.0.lerp(bezier.1, *b);

                let bezier = [
                    bezier.0.position,
                    bezier.0.next_handle.unwrap_or(bezier.0.position),
                    bezier.1.prev_handle.unwrap_or(bezier.1.position),
                    bezier.1.position,
                ];
                trace!("bezier: {:?}", bezier);

                let partial_bezier = trim_cubic_bezier(&bezier, *a, *b);
                trace!("partial_bezier: {:?}", partial_bezier);
                // point_a.position = partial_bezier[0];
                // point_a.next_handle = Some(partial_bezier[1]);
                new_points.last_mut().unwrap().next_handle = Some(partial_bezier[1]);
                point_b.prev_handle = Some(partial_bezier[2]);
                point_b.position = partial_bezier[3];

                new_points.push(point_b);
            }
        }

        // trace!(
        //     "[VPath] new_points: {:?}",
        //     new_points.iter().map(|p| p.position()).collect::<Vec<_>>()
        // );

        self.points = new_points;
        trace!("[VPath] aligned points: {}", self.points.len());

        self
    }
}

impl Alignable for VPath {
    fn is_aligned(&self, target: &Self) -> bool {
        self.points.len() == target.points.len()
    }

    fn align_with(&mut self, target: &mut Self) {
        if self.points.len() > target.points.len() {
            target.align_to(self)
        } else {
            self.align_to(target)
        };
    }
}

impl Interpolatable for VPath {
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

impl Opacity for VPath {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.points.iter_mut().for_each(|p| {
            p.set_opacity(opacity);
        });
        self
    }
}
