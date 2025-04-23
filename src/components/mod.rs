use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use glam::{DVec3, IVec3, dvec3, ivec3};

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

impl<T: Component> AsRef<[T]> for ComponentVec<T> {
    fn as_ref(&self) -> &[T] {
        &self.0
    }
}

impl<T: Component> AsMut<[T]> for ComponentVec<T> {
    fn as_mut(&mut self) -> &mut [T] {
        &mut self.0
    }
}

// MARK: Trait impls

impl<T: Component + PointWise + Interpolatable> Partial for ComponentVec<T> {
    fn get_partial(&self, range: std::ops::Range<f64>) -> Self {
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
    fn lerp(&self, target: &Self, t: f64) -> Self {
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
pub trait PointWise {}

// MARK: Anchor
/// The anchor of the transformation.
#[derive(Debug, Clone, Copy)]
pub enum Anchor {
    /// A point anchor, which is an absolute coordinate
    Point(DVec3),
    /// An edge anchor, use -1, 0, 1 to specify the edge on each axis
    Edge(IVec3),
}

impl Anchor {
    pub fn point(x: f64, y: f64, z: f64) -> Self {
        Self::Point(dvec3(x, y, z))
    }

    pub fn origin() -> Self {
        Self::Point(DVec3::ZERO)
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
    Height(f64),
    /// Scale the mobject to a given width, the height will remain unchanged.
    Width(f64),
    /// Scale the mobject to a given size.
    Size(f64, f64),
    /// Scale the mobject proportionally to a given height, the width will also be scaled accordingly.
    PorportionalHeight(f64),
    /// Scale the mobject proportionally to a given width, the height will also be scaled accordingly.
    PorportionalWidth(f64),
}

// MARK: Test
#[cfg(test)]
mod test {
    use glam::{DVec3, IVec3, dvec3, ivec3};

    use crate::{
        components::vpoint::VPointComponentVec,
        items::{Blueprint, group::Group, vitem::Square},
        traits::{BoundingBox, Position},
    };

    use super::Anchor;

    #[test]
    fn test_bounding_box() {
        let points: VPointComponentVec = VPointComponentVec(
            vec![
                dvec3(-100.0, -100.0, 0.0),
                dvec3(-100.0, 100.0, 0.0),
                dvec3(100.0, 100.0, 0.0),
                dvec3(100.0, -200.0, 0.0),
            ]
            .into(),
        );
        assert_eq!(
            points.get_bounding_box(),
            [
                dvec3(-100.0, -200.0, 0.0),
                dvec3(0.0, -50.0, 0.0),
                dvec3(100.0, 100.0, 0.0)
            ]
        );
        assert_eq!(
            dvec3(0.0, -50.0, 0.0),
            points.get_bounding_box_point(ivec3(0, 0, 0))
        );
        assert_eq!(
            dvec3(-100.0, -200.0, 0.0),
            points.get_bounding_box_point(ivec3(-1, -1, 0))
        );
        assert_eq!(
            dvec3(-100.0, 100.0, 0.0),
            points.get_bounding_box_point(ivec3(-1, 1, 0))
        );
        assert_eq!(
            dvec3(100.0, -200.0, 0.0),
            points.get_bounding_box_point(ivec3(1, -1, 0))
        );
        assert_eq!(
            dvec3(100.0, 100.0, 0.0),
            points.get_bounding_box_point(ivec3(1, 1, 0))
        );
    }

    #[test]
    fn test_transform() {
        let square = vec![
            dvec3(-1.0, -1.0, 0.0),
            dvec3(2.0, -2.0, 0.0),
            dvec3(0.5, 1.0, 0.0),
            dvec3(-3.0, 3.0, 0.0),
            dvec3(4.0, 4.0, 0.0),
        ];
        let mut scale_origin = VPointComponentVec(square.clone().into());
        assert_eq!(
            scale_origin.get_bounding_box_point(IVec3::ZERO),
            dvec3(0.5, 1.0, 0.0)
        );
        scale_origin.scale(DVec3::splat(3.0));

        let ans = VPointComponentVec(
            vec![
                dvec3(-4.0, -5.0, 0.0),
                dvec3(5.0, -8.0, 0.0),
                dvec3(0.5, 1.0, 0.0),
                dvec3(-10.0, 7.0, 0.0),
                dvec3(11.0, 10.0, 0.0),
            ]
            .into(),
        );
        assert_eq!(scale_origin, ans);
    }

    #[test]
    fn test_group_transform() {
        // 20 40 ... 100
        let mut group = (1..=5)
            .map(|i| {
                let mut sq = Square(i as f64 * 20.0).build();
                let x = (0..i).map(|i| i as f64 * 20.0).sum();
                sq.put_anchor_on(Anchor::edge(-1, 0, 0), dvec3(x, 0.0, 0.0));
                sq
            })
            .collect::<Group<_>>();
        assert_eq!(
            group.get_bounding_box(),
            [
                dvec3(0.0, -50.0, 0.0),
                dvec3(150.0, 0.0, 0.0),
                dvec3(300.0, 50.0, 0.0)
            ]
        );
        group.scale(DVec3::splat(2.0));
        assert_eq!(
            group.get_bounding_box(),
            [
                dvec3(-150.0, -100.0, 0.0),
                dvec3(150.0, 0.0, 0.0),
                dvec3(450.0, 100.0, 0.0)
            ]
        );

        assert_eq!(
            group.get(1..).unwrap().get_bounding_box(),
            [
                dvec3(-110.0, -100.0, 0.0),
                dvec3(170.0, 0.0, 0.0),
                dvec3(450.0, 100.0, 0.0)
            ]
        );
        group.get_mut(1..).unwrap().scale(DVec3::splat(0.25));
        assert_eq!(
            group.get(1..).unwrap().get_bounding_box(),
            [
                dvec3(100.0, -25.0, 0.0),
                dvec3(170.0, 0.0, 0.0),
                dvec3(240.0, 25.0, 0.0)
            ]
        )
    }
}
