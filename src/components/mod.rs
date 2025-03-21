use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use glam::{Affine2, IVec3, Mat3, Vec3, Vec3Swizzles, ivec3, vec2, vec3};
use itertools::Itertools;

use crate::{
    prelude::{Alignable, Interpolatable, Partial},
    utils::math::interpolate_usize,
};

pub mod point;
pub mod rgba;
pub mod vpoint;
pub mod nvpoint;
pub mod width;

/// An component
pub trait Component: Debug + Default + Clone + Copy + PartialEq {}

impl<T: Debug + Default + Clone + Copy + PartialEq> Component for T {}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct ComponentVec<T: Component>(Vec<T>);

impl<T: Component> Deref for ComponentVec<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Component> DerefMut for ComponentVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// MARK: Trait impls

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

impl<T: Component> ComponentVec<T> {
    pub fn extend_from_vec(&mut self, vec: Vec<T>) {
        self.0.extend(vec);
    }

    pub fn resize_with_default(&mut self, new_len: usize) {
        self.0.resize(new_len, Default::default());
    }

    pub fn resize_with_last(&mut self, new_len: usize) {
        let last = self.last().cloned().unwrap_or_default();
        self.0.resize(new_len, last);
    }

    pub fn set_all(&mut self, value: impl Into<T>) {
        let value = value.into();
        self.iter_mut().for_each(|x| *x = value);
    }
}

/// A marker trait for components that has each element as a point data.
///
/// For example [`vpoint::VPoint`] is not point wise.
pub trait PointWise {}

// MARK: Transformable
/// A trait about transforming the mobject in 3d space.
///
/// This trait is automatically implemented for `T` that implements [`HasTransform3d`].
/// And for `T` that implements this trait, `[T]` will also implement this trait.
///
/// But should note that, `[T]`'s implementation is not equivalent to doing the same operation on each item, The [`Anchor`] point will be calculated from the bounding box of the whole slice.
pub trait Transformable<T: Transform3dComponent> {
    fn get_start_position(&self) -> Option<Vec3>;
    fn get_end_position(&self) -> Option<Vec3>;
    fn apply_points_function(
        &mut self,
        f: impl Fn(&mut ComponentVec<T>) + Copy,
        anchor: Anchor,
    ) -> &mut Self;

    /// Put anchor at a given point.
    ///
    /// See [`Anchor`] for more details.
    fn put_anchor_on(&mut self, anchor: Anchor, point: Vec3) -> &mut Self {
        let anchor = match anchor {
            Anchor::Edge(edge) => self.get_bounding_box_point(edge),
            Anchor::Point(point) => point,
        };
        self.shift(point - anchor);
        self
    }
    /// Put center at a given point.
    fn put_center_on(&mut self, point: Vec3) -> &mut Self {
        self.put_anchor_on(Anchor::center(), point)
    }
    /// Shift the mobject by a given vector.
    fn shift(&mut self, shift: Vec3) -> &mut Self {
        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    **p += shift;
                });
            },
            Anchor::origin(),
        );
        self
    }
    /// Scale the mobject to a given hint.
    ///
    /// See [`ScaleHint`] for more details.
    fn scale_to(&mut self, hint: ScaleHint) -> &mut Self {
        let bb = self.get_bounding_box();
        let scale = match hint {
            ScaleHint::Height(h) => vec3(1.0, h / (bb[2].y - bb[0].y), 1.0),
            ScaleHint::Width(w) => vec3(w / (bb[2].x - bb[0].x), 1.0, 1.0),
            ScaleHint::Size(w, h) => vec3(w / (bb[2].x - bb[0].x), h / (bb[2].y - bb[0].y), 1.0),
            ScaleHint::PorportionalHeight(h) => Vec3::splat(h / (bb[2].y - bb[0].y)),
            ScaleHint::PorportionalWidth(w) => Vec3::splat(w / (bb[2].x - bb[0].x)),
        };
        self.scale(scale);
        self
    }
    /// Scale the mobject from its center by a given scale vector.
    ///
    /// This is equivalent to [`Transformable::scale_by_anchor`] with [`Anchor::center`].
    fn scale(&mut self, scale: Vec3) -> &mut Self {
        self.scale_by_anchor(scale, Anchor::center())
    }
    /// Scale the mobject by a given vector.
    fn scale_by_anchor(&mut self, scale: Vec3, anchor: Anchor) -> &mut Self {
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
        self.rotate_by_anchor(angle, axis, Anchor::center())
    }
    /// Rotate the mobject by a given angle about a given axis.
    fn rotate_by_anchor(&mut self, angle: f32, axis: Vec3, anchor: Anchor) -> &mut Self {
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
    fn apply_affine(&mut self, affine: Affine2) -> &mut Self {
        self.apply_points_function(
            |points| {
                points.iter_mut().for_each(|p| {
                    let transformed = affine.transform_point2(p.xy());
                    p.x = transformed.x;
                    p.y = transformed.y;
                });
            },
            Anchor::origin(),
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
            Anchor::Point(cur_start),
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
    /// Get the bounding box of the mobject in [min, mid, max] order.
    fn get_bounding_box(&self) -> [Vec3; 3];
    /// Get the bounding box point of the mobject at an edge Anchor.
    ///
    /// See [`Anchor`].
    fn get_bounding_box_point(&self, edge: IVec3) -> Vec3 {
        let bb = self.get_bounding_box();
        let signum = (edge.signum() + IVec3::ONE).as_uvec3();

        vec3(
            bb[signum.x as usize].x,
            bb[signum.y as usize].y,
            bb[signum.z as usize].z,
        )
    }
    /// Get the bounding box corners of the mobject.
    ///
    /// The order is the cartesian product of [-1, 1] on x, y, z axis.
    /// Which is `(-1, -1, -1)`, `(-1, -1, 1)`, `(-1, 1, -1)`, `(-1, 1, 1)`, ...
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

// MARK: [T]
impl<T: Transformable<C>, C: Transform3dComponent> Transformable<C> for [T] {
    fn get_start_position(&self) -> Option<Vec3> {
        self.first().and_then(|x| x.get_start_position())
    }
    fn get_end_position(&self) -> Option<Vec3> {
        self.last().and_then(|x| x.get_end_position())
    }
    fn apply_points_function(
        &mut self,
        f: impl Fn(&mut ComponentVec<C>) + Copy,
        anchor: Anchor,
    ) -> &mut Self {
        let point = match anchor {
            Anchor::Edge(edge) => self.get_bounding_box_point(edge),
            Anchor::Point(point) => point,
        };
        // println!("{:?}, {:?}", anchor, point);
        self.iter_mut().for_each(|x| {
            x.apply_points_function(f, Anchor::Point(point));
        });
        self
    }
    fn get_bounding_box(&self) -> [Vec3; 3] {
        let [min, max] = self
            .iter()
            .map(|x| x.get_bounding_box())
            .map(|[min, _, max]| [min, max])
            .reduce(|a, b| [a[0].min(b[0]), a[1].max(b[1])])
            .unwrap_or([Vec3::ZERO; 2]);
        [min, (min + max) / 2.0, max]
    }
}

// MARK: T
impl<T: HasTransform3d<C>, C: Transform3dComponent> Transformable<C> for T {
    /// Apply a function to the points of the mobject about the point.
    fn apply_points_function(
        &mut self,
        f: impl Fn(&mut ComponentVec<C>) + Copy,
        anchor: Anchor,
    ) -> &mut Self {
        let anchor = match anchor {
            Anchor::Point(x) => x,
            Anchor::Edge(x) => self.get_bounding_box_point(x),
        };
        let component_vec = self.as_mut();

        if anchor != Vec3::ZERO {
            component_vec.iter_mut().for_each(|p| **p -= anchor);
        }

        f(component_vec);

        if anchor != Vec3::ZERO {
            component_vec.iter_mut().for_each(|p| **p += anchor);
        }
        self
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

// MARK: Anchor
/// The anchor of the transformation.
#[derive(Debug, Clone, Copy)]
pub enum Anchor {
    /// A point anchor, which is an absolute coordinate
    Point(Vec3),
    /// An edge anchor, use -1, 0, 1 to specify the edge on each axis
    Edge(IVec3),
}

impl Anchor {
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

// MARK: ScaleTo
/// A hint for scaling the mobject.
pub enum ScaleHint {
    /// Scale the mobject to a given height, the width will remain unchanged.
    Height(f32),
    /// Scale the mobject to a given width, the height will remain unchanged.
    Width(f32),
    /// Scale the mobject to a given size.
    Size(f32, f32),
    /// Scale the mobject proportionally to a given height, the width will also be scaled accordingly.
    PorportionalHeight(f32),
    /// Scale the mobject proportionally to a given width, the height will also be scaled accordingly.
    PorportionalWidth(f32),
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

// MARK: Test

#[cfg(test)]
mod test {
    use glam::{IVec3, Vec3, ivec3, vec3};

    use crate::{
        components::Transformable,
        items::{Blueprint, group::Group, vitem::Square},
    };

    use super::{Anchor, ComponentVec, vpoint::VPoint};

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

    #[test]
    fn test_group_transform() {
        // 20 40 ... 100
        let mut group = (1..=5)
            .map(|i| {
                let mut sq = Square(i as f32 * 20.0).build();
                let x = (0..i).map(|i| i as f32 * 20.0).sum();
                sq.put_anchor_on(Anchor::edge(-1, 0, 0), vec3(x, 0.0, 0.0));
                sq
            })
            .collect::<Group<_>>();
        assert_eq!(
            group.get_bounding_box(),
            [
                vec3(0.0, -50.0, 0.0),
                vec3(150.0, 0.0, 0.0),
                vec3(300.0, 50.0, 0.0)
            ]
        );
        group.scale(Vec3::splat(2.0));
        assert_eq!(
            group.get_bounding_box(),
            [
                vec3(-150.0, -100.0, 0.0),
                vec3(150.0, 0.0, 0.0),
                vec3(450.0, 100.0, 0.0)
            ]
        );

        assert_eq!(
            group.get(1..).unwrap().get_bounding_box(),
            [
                vec3(-110.0, -100.0, 0.0),
                vec3(170.0, 0.0, 0.0),
                vec3(450.0, 100.0, 0.0)
            ]
        );
        group.get_mut(1..).unwrap().scale(Vec3::splat(0.25));
        assert_eq!(
            group.get(1..).unwrap().get_bounding_box(),
            [
                vec3(100.0, -25.0, 0.0),
                vec3(170.0, 0.0, 0.0),
                vec3(240.0, 25.0, 0.0)
            ]
        )
    }
}
