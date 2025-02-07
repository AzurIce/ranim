use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use glam::{ivec3, vec2, vec3, Affine2, IVec3, Mat3, Vec3, Vec3Swizzles};
use itertools::Itertools;

use crate::{
    prelude::{Alignable, Interpolatable, Partial},
    utils::math::interpolate_usize,
};

pub mod point;
pub mod rgba;
pub mod vpoint;
pub mod width;

/// An component
pub trait Component: Debug + Default + Clone + Copy + PartialEq {}

impl<T: Debug + Default + Clone + Copy + PartialEq> Component for T {}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct ComponentVec<T: Component>(Vec<T>);

impl<T: Component + PointWise + Interpolatable> Partial for ComponentVec<T> {
    fn get_partial(&self, range: std::ops::Range<f32>) -> Self {
        let max_idx = self.len() - 2;

        let (start_index, start_residue) = interpolate_usize(0, max_idx, range.start);
        let (end_index, end_residue) = interpolate_usize(0, max_idx, range.end);
        // trace!("max_idx: {max_idx}, range: {:?}, start: {} {}, end: {} {}", range, start_index, start_residue, end_index, end_residue);
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
        }
        .into()
    }
}

impl<T: Component> Alignable for ComponentVec<T> {
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

impl<T: Component + Interpolatable> Interpolatable for ComponentVec<T> {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Self(
            self.iter()
                .zip(target.iter())
                .map(|(a, b)| a.lerp(b, t))
                .collect::<Vec<_>>(),
        )
    }
}

impl<T: Component, I: IntoIterator<Item = impl Into<T>>> From<I> for ComponentVec<T> {
    fn from(v: I) -> Self {
        Self(v.into_iter().map(Into::into).collect())
    }
}

// impl<T: Component> AsRef<[T]> for ComponentVec<T> {
//     fn as_ref(&self) -> &[T] {
//         &self.0
//     }
// }

impl<T: Component> AsRef<Vec<T>> for ComponentVec<T> {
    fn as_ref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T: Component> AsMut<Vec<T>> for ComponentVec<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}

impl<T: Component> Deref for ComponentVec<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Component> DerefMut for ComponentVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Component> ComponentVec<T> {
    pub fn extend_from_vec(&mut self, vec: Vec<T>) {
        self.0.extend(vec);
    }

    pub fn resize_with_default(&mut self, new_len: usize) {
        self.resize(new_len, Default::default());
    }

    pub fn resize_with_last(&mut self, new_len: usize) {
        let last = self.last().cloned().unwrap_or_default();
        self.resize(new_len, last);
    }

    pub fn set_all(&mut self, value: impl Into<T>) {
        let value = value.into();
        self.iter_mut().for_each(|x| *x = value);
    }
}

pub trait PointWise {}

// MARK: Transformable
pub trait Transformable<T: Transform3dComponent> {
    fn get_start_position(&self) -> Option<Vec3>;
    fn get_end_position(&self) -> Option<Vec3>;
    fn apply_points_function(
        &mut self,
        f: impl Fn(&mut ComponentVec<T>) + Copy,
        anchor: TransformAnchor,
    );

    /// Put center at a given point.
    fn put_center_on(&mut self, point: Vec3) -> &mut Self {
        self.shift(point - self.get_bounding_box()[1]);
        self
    }
    /// Shift the mobject by a given vector.
    fn shift(&mut self, shift: Vec3) -> &mut Self {
        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    **p += shift;
                });
            },
            TransformAnchor::origin(),
        );
        self
    }
    /// Scale the mobject by a given vector.
    fn scale(&mut self, scale: Vec3) -> &mut Self {
        self.scale_by_anchor(scale, TransformAnchor::center())
    }
    /// Scale the mobject by a given vector.
    fn scale_by_anchor(&mut self, scale: Vec3, anchor: TransformAnchor) -> &mut Self {
        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    **p *= scale;
                });
            },
            anchor,
        );
        self
    }
    /// Rotate the mobject by a given angle about a given axis.
    fn rotate(&mut self, angle: f32, axis: Vec3) -> &mut Self {
        self.rotate_by_anchor(angle, axis, TransformAnchor::center())
    }
    /// Rotate the mobject by a given angle about a given axis.
    fn rotate_by_anchor(&mut self, angle: f32, axis: Vec3, anchor: TransformAnchor) -> &mut Self {
        let axis = axis.normalize();
        let rotation = Mat3::from_axis_angle(axis, angle);

        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    **p = rotation * **p;
                });
            },
            anchor,
        );
        self
    }
    /// Simple multiplies the matrix to the points.
    fn apply_affine(&mut self, affine: Affine2) {
        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    let transformed = affine.transform_point2(p.xy());
                    p.x = transformed.x;
                    p.y = transformed.y;
                });
            },
            TransformAnchor::origin(),
        )
    }
    /// Put the start and end points of the item on the given points.
    fn put_start_and_end_on(&mut self, start: Vec3, end: Vec3) -> &mut Self {
        let (cur_start, cur_end) = (
            self.get_start_position().unwrap_or_default(),
            self.get_end_position().unwrap_or_default(),
        );
        let cur_v = cur_end - cur_start;
        if cur_v.length_squared() <= f32::EPSILON {
            return self;
        }

        let v = end - start;
        self.scale_by_anchor(
            Vec3::splat(v.length() / cur_v.length()),
            TransformAnchor::Point(cur_start),
        );
        let angle = cur_v.y.atan2(-cur_v.x) - v.y.atan2(-v.x) + std::f32::consts::PI / 2.0;
        self.rotate(angle, Vec3::Z);
        let cur_xy = vec2(cur_v.x, cur_v.y);
        let cur_xy = cur_xy * cur_xy.abs().normalize();

        let xy = vec2(v.x, v.y);
        let xy = xy * xy.abs().normalize();
        let angle = cur_v.z.atan2(-cur_xy.length()) - v.z.atan2(-xy.length());
        self.rotate(angle, vec3(-v.y, v.x, 0.0));
        self.shift(start - self.get_start_position().unwrap());

        self
    }

    // Bounding box
    fn get_bounding_box(&self) -> [Vec3; 3];
    fn get_bounding_box_point(&self, edge: IVec3) -> Vec3 {
        let bb = self.get_bounding_box();
        let signum = (edge.signum() + IVec3::ONE).as_uvec3();

        vec3(
            bb[signum.x as usize].x,
            bb[signum.y as usize].y,
            bb[signum.z as usize].z,
        )
    }
    fn get_bounding_box_corners(&self) -> [Vec3; 8] {
        [-1, 1]
            .into_iter()
            .cartesian_product([-1, 1])
            .cartesian_product([-1, 1])
            .map(|((x, y), z)| self.get_bounding_box_point(ivec3(x, y, z)))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

impl<T: Transform3dComponent, V: HasTransform3d<T>> Transformable<T> for V {
    /// Apply a function to the points of the mobject about the point.
    fn apply_points_function(
        &mut self,
        f: impl Fn(&mut ComponentVec<T>) + Copy,
        anchor: TransformAnchor,
    ) {
        let anchor = match anchor {
            TransformAnchor::Point(x) => x,
            TransformAnchor::Edge(x) => self.get_bounding_box_point(x),
        };
        let component_vec = self.as_mut();

        if anchor != Vec3::ZERO {
            component_vec.iter_mut().for_each(|p| **p -= anchor);
        }

        f(component_vec);

        if anchor != Vec3::ZERO {
            component_vec.iter_mut().for_each(|p| **p += anchor);
        }
    }

    fn get_start_position(&self) -> Option<Vec3> {
        self.as_ref().first().map(|&p| *p)
    }

    fn get_end_position(&self) -> Option<Vec3> {
        self.as_ref().last().map(|&p| *p)
    }

    /// Get the bounding box of the mobject.
    /// min, mid, max
    fn get_bounding_box(&self) -> [Vec3; 3] {
        let min = self
            .as_ref()
            .iter()
            .map(|&p| *p)
            .reduce(|acc, e| acc.min(e))
            .unwrap_or(Vec3::ZERO);
        let max = self
            .as_ref()
            .iter()
            .map(|&p| *p)
            .reduce(|acc, e| acc.max(e))
            .unwrap_or(Vec3::ZERO);
        let mid = (min + max) / 2.0;
        [min, mid, max]
    }
}

/// The anchor of the transformation.
#[derive(Debug, Clone, Copy)]
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

    pub fn center() -> Self {
        Self::Edge(IVec3::ZERO)
    }

    pub fn edge(x: i32, y: i32, z: i32) -> Self {
        Self::Edge(ivec3(x, y, z))
    }
}

// MARK: Transform3d
/// An `Transform3dComponent` should be a `Vec3` that can be transformed.
pub trait Transform3dComponent: Component + DerefMut<Target = Vec3> {}
impl<T: Component + DerefMut<Target = Vec3>> Transform3dComponent for T {}

/// Something that can be treated as a ref/mut ref of `ComponentVec<Transform3dComponent>`.
pub trait HasTransform3d<T: Transform3dComponent>:
    AsRef<ComponentVec<T>> + AsMut<ComponentVec<T>>
{
}
impl<T: Transform3dComponent, V: AsRef<ComponentVec<T>> + AsMut<ComponentVec<T>>> HasTransform3d<T>
    for V
{
}

impl<T: Component> AsRef<ComponentVec<T>> for ComponentVec<T> {
    fn as_ref(&self) -> &ComponentVec<T> {
        self
    }
}
impl<T: Component> AsMut<ComponentVec<T>> for ComponentVec<T> {
    fn as_mut(&mut self) -> &mut ComponentVec<T> {
        self
    }
}

#[cfg(test)]
mod test {
    use glam::{ivec3, vec3, IVec3, Vec3};

    use crate::components::Transformable;

    use super::{vpoint::VPoint, ComponentVec};

    #[test]
    fn test_bounding_box() {
        let points: ComponentVec<VPoint> = vec![
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
            vec3(0.0, -50.0, 0.0),
            points.get_bounding_box_point(ivec3(0, 0, 0))
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

    #[test]
    fn test_transform() {
        let square = vec![
            vec3(-1.0, -1.0, 0.0),
            vec3(2.0, -2.0, 0.0),
            vec3(0.5, 1.0, 0.0),
            vec3(-3.0, 3.0, 0.0),
            vec3(4.0, 4.0, 0.0),
        ];
        let mut scale_origin: ComponentVec<VPoint> = square.clone().into();
        assert_eq!(
            scale_origin.get_bounding_box_point(IVec3::ZERO),
            vec3(0.5, 1.0, 0.0)
        );
        scale_origin.scale(Vec3::splat(3.0));

        let ans: ComponentVec<VPoint> = vec![
            vec3(-4.0, -5.0, 0.0),
            vec3(5.0, -8.0, 0.0),
            vec3(0.5, 1.0, 0.0),
            vec3(-10.0, 7.0, 0.0),
            vec3(11.0, 10.0, 0.0),
        ]
        .into();
        assert_eq!(scale_origin, ans);
    }
}
