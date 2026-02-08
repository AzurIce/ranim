use std::fmt::Debug;

use derive_more::{Deref, DerefMut, From};

use crate::{
    prelude::{Alignable, Interpolatable},
    utils::{math::interpolate_usize, resize_preserving_order},
};

/// A Vec of Pointwise data
#[derive(Debug, PartialEq, Eq, Deref, DerefMut, From)]
pub struct PointVec<T>(Vec<T>);

impl<T: Interpolatable> Interpolatable for PointVec<T> {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self(self.0.lerp(&target.0, t))
    }
}

impl<T: Clone> Clone for PointVec<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Component + Interpolatable> PointVec<T> {
    /// Get a partial PointVec within a specified range.
    ///
    /// This will interpolate the values at the start and end indices, and then
    /// return a new PointVec containing the interpolated values.
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

/// Vec resizing utils
pub trait VecResizeTrait {
    /// Resize with default value
    fn resize_with_default(&mut self, new_len: usize);
    /// Resize with last element
    fn resize_with_last(&mut self, new_len: usize);
    /// Resize preserved order
    fn resize_preserving_order(&mut self, new_len: usize);
}

impl<T: Component> VecResizeTrait for Vec<T> {
    /// Resize with default value
    fn resize_with_default(&mut self, new_len: usize) {
        self.resize(new_len, Default::default());
    }
    /// Resize with last element
    fn resize_with_last(&mut self, new_len: usize) {
        let last = self.last().cloned().unwrap_or_default();
        self.resize(new_len, last);
    }
    /// Resize preserved order
    fn resize_preserving_order(&mut self, new_len: usize) {
        *self = resize_preserving_order(self, new_len);
    }
}

impl<T: Component> Alignable for PointVec<T> {
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

// MARK: Test
#[cfg(test)]
mod test {
    use glam::{DVec3, dvec3};

    use crate::{
        anchor::{Aabb, AabbPoint, Locate},
        components::vpoint::VPointVec,
        traits::ScaleExt,
    };

    #[test]
    fn test_bounding_box() {
        let points: VPointVec = VPointVec(vec![
            dvec3(-100.0, -100.0, 0.0),
            dvec3(-100.0, 100.0, 0.0),
            dvec3(100.0, 100.0, 0.0),
            dvec3(100.0, -200.0, 0.0),
        ]);
        assert_eq!(
            points.aabb(),
            [dvec3(-100.0, -200.0, 0.0), dvec3(100.0, 100.0, 0.0)]
        );
        assert_eq!(
            dvec3(0.0, -50.0, 0.0),
            AabbPoint(dvec3(0.0, 0.0, 0.0)).locate(&points)
        );
        assert_eq!(
            dvec3(-100.0, -200.0, 0.0),
            AabbPoint(dvec3(-1.0, -1.0, 0.0)).locate(&points)
        );
        assert_eq!(
            dvec3(-100.0, 100.0, 0.0),
            AabbPoint(dvec3(-1.0, 1.0, 0.0)).locate(&points)
        );
        assert_eq!(
            dvec3(100.0, -200.0, 0.0),
            AabbPoint(dvec3(1.0, -1.0, 0.0)).locate(&points)
        );
        assert_eq!(
            dvec3(100.0, 100.0, 0.0),
            AabbPoint(dvec3(1.0, 1.0, 0.0)).locate(&points)
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
        let mut scale_origin = VPointVec(square.clone());
        assert_eq!(
            AabbPoint(DVec3::ZERO).locate(&scale_origin),
            dvec3(0.5, 1.0, 0.0)
        );
        scale_origin.scale_at(DVec3::splat(3.0), AabbPoint::CENTER);

        let ans = VPointVec(vec![
            dvec3(-4.0, -5.0, 0.0),
            dvec3(5.0, -8.0, 0.0),
            dvec3(0.5, 1.0, 0.0),
            dvec3(-10.0, 7.0, 0.0),
            dvec3(11.0, 10.0, 0.0),
        ]);
        assert_eq!(scale_origin, ans);
    }
}
