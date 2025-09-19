use std::fmt::Debug;

use derive_more::{AsMut, AsRef, Deref, DerefMut};

use crate::{
    prelude::{Alignable, Interpolatable},
    utils::{math::interpolate_usize, resize_preserving_order},
};

/// Point
pub mod point;
/// Rgba
pub mod rgba;
/// Vpoint
pub mod vpoint;
/// Width
pub mod width;

/// An component
pub trait Component: Debug + Default + Clone + PartialEq {}

impl<T: Debug + Default + Clone + PartialEq> Component for T {}

pub trait HasComponent<T: Component> {
    fn get_component(&self) -> &T;
    fn set_component_mut(&mut self) -> &mut T;
}

/// A component vec
#[derive(Default, Debug, Clone, PartialEq, Deref, DerefMut, AsMut, AsRef)]
#[as_ref(forward)]
#[as_mut(forward)]
pub struct ComponentVec<T: Component>(pub(crate) Vec<T>);

// MARK: Trait impls

impl<T: Component + PointWise + Interpolatable> ComponentVec<T> {
    /// Get partial of data
    ///
    /// This will interpolate between point wise data
    pub fn get_partial(&self, range: std::ops::Range<f64>) -> Self {
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

impl<T: Component> ComponentVec<T> {
    /// Extend from a vec
    pub fn extend_from_vec(&mut self, vec: Vec<T>) {
        self.0.extend(vec);
    }
    /// Resize with default value
    pub fn resize_with_default(&mut self, new_len: usize) {
        self.0.resize(new_len, Default::default());
    }
    /// Resize with last element
    pub fn resize_with_last(&mut self, new_len: usize) {
        let last = self.last().cloned().unwrap_or_default();
        self.0.resize(new_len, last);
    }
    /// Resize preserved order
    pub fn resize_preserving_order(&mut self, new_len: usize) {
        self.0 = resize_preserving_order(&self.0, new_len);
    }
    /// Set all element to a value
    pub fn set_all(&mut self, value: impl Into<T>) {
        let value = value.into();
        self.iter_mut().for_each(|x| *x = value.clone());
    }
}

/// A marker trait for components that has each element as a point data.
pub trait PointWise {}

// MARK: Test
#[cfg(test)]
mod test {
    use glam::{DVec3, IVec3, dvec3, ivec3};

    use crate::{
        components::vpoint::VPointComponentVec,
        traits::{BoundingBox, Scale},
    };

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
}
