use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use glam::{ivec3, vec2, vec3, IVec3, Mat3, Vec3};
use log::trace;

use crate::{
    prelude::{Alignable, Interpolatable, Partial},
    utils::math::interpolate_usize,
};

pub mod point;
pub mod rgba;
pub mod vpoint;
pub mod width;

#[derive(Default, Clone)]
pub struct ComponentData<T: Default + Clone>(Vec<T>);

// impl<T: Default + Clone> Partial for ComponentData<T> {
//     fn get_partial(&self, range: std::ops::Range<f32>) -> Self {
//         let start = (range.start * self.len() as f32).floor() as usize;
//         let end = (range.end * self.len() as f32).floor() as usize;
//         Self(self.get(start..end).unwrap().to_vec())
//     }
// }

impl<T: Default + Clone + PointWise + Interpolatable + Debug> Partial for ComponentData<T> {
    fn get_partial(&self, range: std::ops::Range<f32>) -> Self {
        let max_idx = self.len() - 1;
        let (start_index, start_residue) = interpolate_usize(0, max_idx, range.start);
        let (end_index, end_residue) = interpolate_usize(0, max_idx, range.end);
        // trace!("range: {:?}, start: {} {}, end: {} {}", range, start_index, start_residue, end_index, end_residue);
        if start_index == end_index {
            let start_v = self
                .get(start_index)
                .unwrap()
                .lerp(self.get(start_index + 1).unwrap(), start_residue);
            let end_v = self
                .get(end_index)
                .unwrap()
                .lerp(self.get(end_index + 1).unwrap(), end_residue);
            vec![start_v, end_v]
        } else {
            let start_v = self
                .get(start_index)
                .unwrap()
                .lerp(self.get(start_index + 1).unwrap(), start_residue);
            let end_v = self
                .get(end_index)
                .unwrap()
                .lerp(self.get(end_index + 1).unwrap(), end_residue);

            let mut partial = Vec::with_capacity(end_index - start_index + 1 + 2);
            partial.push(start_v);
            partial.extend_from_slice(self.get(start_index + 1..=end_index).unwrap());
            partial.push(end_v);
            partial
        }.into()
    }
}

impl<T: Default + Clone> Alignable for ComponentData<T> {
    fn is_aligned(&self, other: &Self) -> bool {
        self.len() == other.len()
    }
    fn align_with(&mut self, other: &mut Self) {
        if self.len() == other.len() {
            return;
        }
        if self.len() < other.len() {
            self.resize_with_last(other.len());
        } else {
            other.resize_with_last(self.len());
        }
    }
}

impl<T: Default + Clone + Interpolatable> Interpolatable for ComponentData<T> {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Self(
            self.iter()
                .zip(target.iter())
                .map(|(a, b)| a.lerp(b, t))
                .collect::<Vec<_>>(),
        )
    }
}

impl<T: Default + Clone + Debug> Debug for ComponentData<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Default + Clone, I: IntoIterator<Item = impl Into<T>>> From<I> for ComponentData<T> {
    fn from(v: I) -> Self {
        Self(v.into_iter().map(Into::into).collect())
    }
}

impl<T: Default + Clone> AsRef<[T]> for ComponentData<T> {
    fn as_ref(&self) -> &[T] {
        &self.0
    }
}

impl<T: Default + Clone> AsRef<Vec<T>> for ComponentData<T> {
    fn as_ref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T: Default + Clone> AsMut<Vec<T>> for ComponentData<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}

impl<T: Default + Clone> Deref for ComponentData<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Default + Clone> DerefMut for ComponentData<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Default + Clone> ComponentData<T> {
    pub fn extend_from_vec(&mut self, vec: Vec<T>) {
        self.0.extend(vec);
    }
}

impl<T: Default + Clone> ComponentData<T> {
    pub fn resize_with_default(&mut self, new_len: usize) {
        self.resize(new_len, Default::default());
    }

    pub fn resize_with_last(&mut self, new_len: usize) {
        let last = self.last().cloned().unwrap_or_default();
        self.resize(new_len, last);
    }

    pub fn set_all(&mut self, value: impl Into<T>) {
        let value = value.into();
        self.iter_mut().for_each(|x| *x = value.clone());
    }
}

pub trait PointWise {}

// MARK: Transformable
pub trait Transformable {
    fn shift(&mut self, offset: Vec3) -> &mut Self;
    fn rotate(&mut self, angle: f32, axis: Vec3, anchor: TransformAnchor) -> &mut Self;
    fn scale(&mut self, scale: Vec3) -> &mut Self;
}

/// The anchor of the transformation.
pub enum TransformAnchor {
    /// A point anchor
    Point(Vec3),
    /// An edge anchor, use -1, 0, 1 to specify the edge on each axis
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

// MARK: Transform3d
pub trait HasTransform3d<T: Transform3d + Default + Clone> {
    fn get(&self) -> &ComponentData<T>;
    fn get_mut(&mut self) -> &mut ComponentData<T>;
}

pub trait Transform3d {
    fn position(&self) -> Vec3;
    fn position_mut(&mut self) -> &mut Vec3;
}

impl<T: Transform3d + Default + Clone> ComponentData<T> {
    pub fn get_start_position(&self) -> Option<Vec3> {
        self.first().map(|p| p.position())
    }

    pub fn get_end_position(&self) -> Option<Vec3> {
        self.last().map(|p| p.position())
    }

    /// Get the bounding box of the mobject.
    /// min, mid, max
    pub fn get_bounding_box(&self) -> [Vec3; 3] {
        let min = self
            .iter()
            .map(|p| p.position())
            .reduce(|acc, e| acc.min(e))
            .unwrap_or(Vec3::ZERO);
        let max = self
            .iter()
            .map(|p| p.position())
            .reduce(|acc, e| acc.max(e))
            .unwrap_or(Vec3::ZERO);
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
        f: impl Fn(&mut ComponentData<T>),
        anchor: TransformAnchor,
    ) {
        let anchor = match anchor {
            TransformAnchor::Point(x) => x,
            TransformAnchor::Edge(x) => self.get_bounding_box_point(x),
        };

        if anchor != Vec3::ZERO {
            self.iter_mut()
                .for_each(|p| *p.position_mut() = p.position() + anchor);
        }

        f(self);

        if anchor != Vec3::ZERO {
            self.iter_mut()
                .for_each(|p| *p.position_mut() = p.position() - anchor);
        }
    }

    /// Shift the mobject by a given vector.
    pub fn shift(&mut self, shift: Vec3) -> &mut Self {
        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    *p.position_mut() = p.position() + shift;
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
                    *p.position_mut() = p.position() * scale;
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
                    *p.position_mut() = rotation * p.position();
                });
            },
            anchor,
        );
        self
    }

    /// Put the start and end points of the item on the given points.
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

#[cfg(test)]
mod test {
    use glam::{ivec3, vec3};

    use super::{vpoint::VPoint, ComponentData};

    #[test]
    fn test_bounding_box() {
        let points: ComponentData<VPoint> = vec![
            vec3(-100.0, -100.0, 0.0),
            vec3(-100.0, 100.0, 0.0),
            vec3(100.0, 100.0, 0.0),
            vec3(100.0, -200.0, 0.0),
        ]
        .into();
        assert_eq!(
            points.get_bounding_box(),
            [
                vec3(-100.0, -200.0, 0.0),
                vec3(0.0, -50.0, 0.0),
                vec3(100.0, 100.0, 0.0)
            ]
        );
        assert_eq!(
            vec3(-100.0, -200.0, 0.0),
            points.get_bounding_box_point(ivec3(-1, -1, 0))
        );
        assert_eq!(
            vec3(-100.0, 100.0, 0.0),
            points.get_bounding_box_point(ivec3(-1, 1, 0))
        );
        assert_eq!(
            vec3(100.0, -200.0, 0.0),
            points.get_bounding_box_point(ivec3(1, -1, 0))
        );
        assert_eq!(
            vec3(100.0, 100.0, 0.0),
            points.get_bounding_box_point(ivec3(1, 1, 0))
        );
    }
}
