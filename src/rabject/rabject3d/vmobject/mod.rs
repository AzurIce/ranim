mod blueprint;
pub mod pipeline;
pub mod primitive;

use std::{cmp::Ordering, fmt::Debug};

use bevy_color::{LinearRgba, Srgba};
pub use blueprint::*;

use glam::{vec2, vec3, IVec3, Mat3, Vec3, Vec4};
use itertools::Itertools;
use primitive::{ExtractedVMobject, VMobjectPrimitive};

use crate::{
    interpolate::Interpolatable,
    prelude::{Alignable, Opacity},
    rabject::TransformAnchor,
    utils::{bezier::partial_quadratic_bezier, rotation_between_vectors},
};

use super::Rabject;

#[allow(unused)]
use log::{info, trace, warn};

#[repr(C, align(16))]
#[derive(Clone, Copy, Default, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VMobjectPoint {
    pub pos: Vec3,
    pub stroke_width: f32,
    pub stroke_color: LinearRgba,
    pub fill_color: LinearRgba,
}

impl Interpolatable for VMobjectPoint {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Self {
            pos: self.pos.lerp(target.pos, t),
            stroke_color: self.stroke_color.lerp(&target.stroke_color, t),
            stroke_width: self.stroke_width.lerp(&target.stroke_width, t),
            fill_color: self.fill_color.lerp(&target.fill_color, t),
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
}

#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct VMobjectFillVertex {
    pub pos: Vec3,
    pub fill_all: u32,
    pub fill_color: LinearRgba,
}

#[repr(C, align(16))]
#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VMobjectStrokeVertex {
    pub pos: Vec4,
    pub stroke_color: Vec4,
}

/// The VMobject is actually a list of quadratic bezier curves.
#[derive(Debug, Clone, Default)]
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
        // println!("points: {:?}", points);

        let anchors = points;
        let handles = anchors
            .iter()
            .zip(anchors.iter().skip(1))
            .map(|(&a, &b)| 0.5 * (a + b))
            .collect::<Vec<_>>();

        // Interleave anchors and handles
        let points = anchors
            .into_iter()
            .interleave(handles.into_iter())
            .collect();
        // println!("points: {:?}", points);

        Self::from_points(points)
    }

    pub fn from_points(points: Vec<Vec3>) -> Self {
        assert!(points.len() % 2 == 1); // must be odd number of points

        let points = points
            .into_iter()
            .map(|p| VMobjectPoint {
                pos: p,
                stroke_color: Srgba::new(1.0, 0.0, 0.0, 1.0).into(),
                stroke_width: 1.0,
                fill_color: Srgba::new(0.0, 0.0, 1.0, 0.5).into(),
            })
            .collect();

        Self { points }
    }

    pub fn is_closed(&self) -> bool {
        if self.points.is_empty() {
            return false;
        }

        (self.points.first().unwrap().pos - self.points.last().unwrap().pos).length() < f32::EPSILON
    }

    pub fn points(&self) -> &[VMobjectPoint] {
        &self.points
    }

    pub fn set_points(&mut self, points: Vec<VMobjectPoint>) {
        self.points = points;
    }

    /// Get the unit normal if it's a polygon.
    ///
    /// <https://stackoverflow.com/questions/22838071/robust-polygon-normal-calculation>
    /// <https://www.khronos.org/opengl/wiki/Calculating_a_Surface_Normal>
    /// <https://github.com/MIERUNE/earcut-rs/blob/3898cc009723bbef827ba6ce1339c240ece52484/src/utils3d.rs#L9>
    pub fn get_unit_normal(&self) -> Vec3 {
        if !self.is_closed() || self.points.len() < 5 {
            return Vec3::Z;
        }

        let anchors = self
            .points
            .iter()
            .step_by(2)
            .map(|p| p.position())
            .collect::<Vec<_>>();

        let normal = anchors
            .iter()
            .skip(1)
            .fold((Vec3::ZERO, anchors[0]), |(acc, pre), &e| {
                (acc + pre.cross(e), e)
            })
            .0;
        if normal.length() < f32::EPSILON {
            return Vec3::Z;
        }
        normal.normalize()
    }

    // TODO: do this in compute shader
    pub fn parse_fill(&self) -> Vec<VMobjectFillVertex> {
        let points = &self.points;
        if points.is_empty() || !self.is_closed() {
            return vec![VMobjectFillVertex::default(); 3];
        }

        let mut vertices = Vec::with_capacity(points.len() * 3); // not acurate
        let base_point = points.first().unwrap();
        // let unit_normal = self.get_unit_normal();
        points
            .iter()
            .step_by(2)
            .zip(points.iter().skip(1).step_by(2))
            .zip(points.iter().skip(2).step_by(2))
            .for_each(|((p0, p1), p2)| {
                let v1 = p0.pos - base_point.pos;
                let v2 = p2.pos - base_point.pos;
                // let mat = Mat3::from_cols(unit_normal, v1, v2);
                // let face = mat.determinant();

                let normal = v1.cross(v2).normalize();
                if normal == Vec3::ZERO {
                    return;
                }
                vertices.extend_from_slice(&[(*base_point, 1), (*p0, 1), (*p2, 1)]);
                vertices.extend_from_slice(&[(*p0, 0), (*p1, 0), (*p2, 0)]);
            });
        vertices
            .into_iter()
            .map(|(v, fill_all)| VMobjectFillVertex {
                pos: v.pos,
                fill_all,
                fill_color: v.fill_color,
            })
            .collect()
    }
}

impl Rabject for VMobject {
    type RenderData = ExtractedVMobject;
    type RenderResource = VMobjectPrimitive;

    fn extract(&self) -> Self::RenderData {
        ExtractedVMobject {
            points: self.points.clone(),
            joint_angles: self.get_joint_angles(),
            unit_normal: self.get_unit_normal(),
            fill_triangles: self.parse_fill(),
        }
    }
}

impl VMobject {
    pub fn get_anchor_position(&self, anchor_index: usize) -> Vec3 {
        self.points[anchor_index * 2].position()
    }
    pub fn get_point_handles_position(&self, handle_index: usize) -> (Option<Vec3>, Option<Vec3>) {
        let index = handle_index * 2;
        let (mut hpre, mut hnext) = if index == 0 {
            (None, Some(self.points[index + 1].position()))
        } else if index == self.points.len() - 1 {
            (Some(self.points[index - 1].position()), None)
        } else {
            (
                Some(self.points[index - 1].position()),
                Some(self.points[index + 1].position()),
            )
        };
        if self.is_closed() {
            if index == 0 {
                hpre = Some(self.points[self.points.len() - 2].position());
            } else if index == self.points.len() - 1 {
                hnext = Some(self.points[1].position());
            }
        }
        (hpre, hnext)
    }
    pub fn get_joint_angles(&self) -> Vec<f32> {
        assert!(self.points.len() >= 3);
        let mut joint_angles = (0..self.points.len() / 2)
            .map(|anchor_index| {
                let (hpre, hnext) = self.get_point_handles_position(anchor_index);

                let (Some(hpre), Some(hnext)) = (hpre, hnext) else {
                    return 0.0;
                };
                let anchor = self.get_anchor_position(anchor_index);
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
        joint_angles.push(if self.is_closed() {
            joint_angles[0]
        } else {
            0.0
        });
        // trace!("[VMobject::get_joint_angles] {:?}", joint_angles);
        joint_angles
    }

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

        vec3(
            bb[signum.x as usize].x,
            bb[signum.y as usize].y,
            bb[signum.z as usize].z,
        )
    }

    /// Get the area vector of the polygon of anchors.
    pub fn get_area_vector(&self) -> Vec3 {
        // trace!(
        //     "[VMobject::get_area_vector] points: {}, closed: {}",
        //     self.points.len(),
        //     self.is_closed()
        // );
        if self.points.is_empty() || !self.is_closed() {
            return Vec3::ZERO;
        }

        let anchors = self.points.iter().step_by(2).collect::<Vec<_>>();

        let sum_and_diffs = anchors
            .iter()
            .zip(anchors.iter().skip(1).chain(anchors.iter().take(1)))
            .map(|(p0, p1)| (p0.position() + p1.position(), p1.position() - p0.position()))
            .collect::<Vec<_>>();

        let x = sum_and_diffs
            .iter()
            .map(|(sum, diff)| sum.y * diff.z)
            .sum::<f32>();
        let y = sum_and_diffs
            .iter()
            .map(|(sum, diff)| sum.z * diff.x)
            .sum::<f32>();
        let z = sum_and_diffs
            .iter()
            .map(|(sum, diff)| sum.x * diff.y)
            .sum::<f32>();

        0.5 * vec3(x, y, z)
    }

    // pub fn get_unit_normal(&self) -> Vec3 {
    //     // trace!("[VMobject::get_unit_normal] points: {:?}", self.points.len());
    //     if self.points.len() < 3 {
    //         return Vec3::Z;
    //     }
    //     let area_vector = self.get_area_vector();
    //     // trace!("[VMobject::get_unit_normal] area_vector: {:?}", area_vector);
    //     if area_vector == Vec3::ZERO {
    //         // warn!("[VMobject] area_vector is zero");
    //         let v1 = (self.points[1].position() - self.points[0].position()).normalize();
    //         let v2 = (self.points[2].position() - self.points[0].position()).normalize();
    //         // trace!("v1: {:?}, v2: {:?}", v1, v2);
    //         // trace!("cross: {:?}", v1.cross(v2));
    //         let v = v1.cross(v2);

    //         // TODO: fix this
    //         return if v.is_nan() || v == Vec3::ZERO {
    //             // warn!("v is nan or zero {:?}, use Z", v);
    //             Vec3::Z
    //         } else {
    //             v.normalize()
    //         };
    //     }
    //     area_vector.normalize()
    // }

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
    pub fn set_stroke_width(&mut self, width: f32) -> &mut Self {
        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    p.set_stroke_width(width);
                });
            },
            TransformAnchor::origin(),
        );
        self
    }
    pub fn set_stroke_color(&mut self, color: impl Into<LinearRgba> + Debug + Copy) -> &mut Self {
        // let color = vec4(color.red, color.green, color.blue, color.alpha);

        self.points.iter_mut().for_each(|p| {
            p.set_stroke_color(color);
        });
        self
    }
    pub fn set_fill_color(&mut self, color: impl Into<LinearRgba> + Debug + Copy) -> &mut Self {
        trace!("set fill color: {:?}", color);
        // let color = vec4(color.red, color.green, color.blue, color.alpha);

        self.points.iter_mut().for_each(|p| p.set_fill_color(color));
        self
    }
    pub fn set_color(&mut self, color: impl Into<LinearRgba> + Debug + Copy) -> &mut Self {
        trace!("[VMobject] set_color: {:?}", color);
        self.set_fill_color(color).set_stroke_color(color);
        self
    }
    pub fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.points.iter_mut().for_each(|p| {
            p.fill_color.alpha = opacity;
            p.stroke_color.alpha = opacity;
        });
        self
    }
}

impl Opacity for VMobject {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.set_opacity(opacity);
        self
    }
}

impl Alignable for VMobject {
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

impl VMobject {
    // pub fn resize_points(&mut self, len: usize) {
    //     if self.points.len() == len {
    //         return;
    //     }
    //     if self.points.len() < len {
    //         extend_with_last(&mut self.points, len);
    //     } else {
    //         self.points = resize_preserving_order(&self.points, len);
    //     }
    // }

    // pub fn aligned_with_rabject(&self, target: &Self) -> bool {
    //     self.points.len() == target.points.len()
    // }

    // pub fn align_with_rabject(&mut self, target: &mut Self) {
    //     let max_len = self.points.len().max(target.points.len());
    //     self.resize_points(max_len);
    //     target.resize_points(max_len);
    // }

    /// Align the mobject to the target mobject.
    pub fn align_to(&mut self, target: &Self) -> &mut Self {
        if self.points.len() >= target.points.len() {
            return self;
        }

        trace!(
            "[VMobject] {} align to {}",
            self.points.len(),
            target.points.len()
        );
        // trace!(
        //     "[VMobject] self: {:?}",
        //     self.points.iter().map(|p| p.position()).collect::<Vec<_>>()
        // );
        // trace!(
        //     "[VMobject] target: {:?}",
        //     target
        //         .points
        //         .iter()
        //         .map(|p| p.position())
        //         .collect::<Vec<_>>()
        // );

        let beziers = self
            .points
            .iter()
            .step_by(2)
            .zip(self.points.iter().skip(1).step_by(2))
            .zip(self.points.iter().skip(2).step_by(2))
            .map(|((&p0, &p1), &p2)| [p0, p1, p2])
            .collect::<Vec<_>>();

        let mut lens = beziers
            .iter()
            .map(|b| (b[2].position() - b[0].position()).length())
            .collect::<Vec<_>>();

        let n = (target.points.len() - self.points.len()) / 2;
        let mut ipc = vec![0; beziers.len()];
        for _ in 0..n {
            let i = lens
                .iter()
                .position_max_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
                .unwrap();
            ipc[i] += 1;
            lens[i] *= ipc[i] as f32 / (ipc[i] + 1) as f32;
        }

        let mut new_points = vec![self.points[0]];
        for (bezier, ipc) in beziers.iter().zip(ipc.into_iter()) {
            let alphas = (0..ipc + 2)
                .map(|i| i as f32 / (ipc + 1) as f32)
                .collect::<Vec<_>>();

            for (a, b) in alphas.iter().zip(alphas.iter().skip(1)) {
                // trace!("[VMobject] a: {}, b: {}", *a, *b);
                let bezier = partial_quadratic_bezier(bezier, *a, *b);
                // trace!(
                //     "[VMobject] bezier: {:?}",
                //     bezier.iter().map(|p| p.position()).collect::<Vec<_>>()
                // );
                new_points.extend(bezier.iter().skip(1));
            }
        }

        // trace!(
        //     "[VMobject] new_points: {:?}",
        //     new_points.iter().map(|p| p.position()).collect::<Vec<_>>()
        // );

        self.points = new_points;
        trace!("[VMobject] aligned points: {}", self.points.len());

        self
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
