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

impl<T: Clone> PointVec<T> {
    /// Set all pointwise values from a slice.
    pub fn set(&mut self, values: &[T]) {
        self.0 = values.to_vec();
    }
}

impl<T: Component + Interpolatable> PointVec<T> {
    /// Sample the values at position `idx` in a target sequence of length `total`.
    ///
    /// A single value is treated as uniform; two or more values are treated as
    /// evenly spaced key values and interpolated linearly.
    pub fn sample(&self, idx: usize, total: usize) -> T {
        match self.len() {
            0 => T::default(),
            1 => self[0].clone(),
            len => {
                if total <= 1 {
                    return self[0].clone();
                }
                let t = idx as f64 / (total - 1) as f64;
                let pos = t * (len - 1) as f64;
                let lo = (pos.floor() as usize).min(len - 2);
                let frac = pos - lo as f64;
                self[lo].lerp(&self[lo + 1], frac)
            }
        }
    }

    /// Expand values to exactly `len` entries by linear sampling.
    pub fn expand_to(&self, len: usize) -> Vec<T> {
        (0..len).map(|idx| self.sample(idx, len)).collect()
    }

    /// Get a partial PointVec within a specified range.
    ///
    /// This will interpolate the values at the start and end indices, and then
    /// return a new PointVec containing the interpolated values.
    pub fn get_partial(&self, range: std::ops::Range<f64>) -> Self {
        if self.len() <= 1 {
            return self.clone();
        }

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
        components::{PointVec, vpoint::VPointVec, width::Width},
        traits::{ScaleTransform, ShiftTransformExt},
    };

    #[test]
    fn point_vec_samples_uniform_values() {
        let widths: PointVec<Width> = vec![2.0.into()].into();
        let expanded = widths.expand_to(4);

        assert_eq!(expanded, vec![2.0.into(); 4]);
    }

    #[test]
    fn point_vec_samples_between_end_values() {
        let widths: PointVec<Width> = vec![0.0.into(), 2.0.into()].into();
        let expanded = widths.expand_to(3);

        assert_eq!(expanded, vec![0.0.into(), 1.0.into(), 2.0.into()]);
    }

    #[test]
    fn point_vec_partial_accepts_single_value() {
        let widths: PointVec<Width> = vec![2.0.into()].into();

        assert_eq!(widths.get_partial(0.25..0.75), widths);
    }

    #[test]
    fn test_bounding_box() {
        // 5 points = 2 bezier segments: [P0,P1,P2] and [P2,P3,P4]
        // P3 == P2 signals a subpath break, so 2nd segment is a line from P2 to P4.
        let points: VPointVec = VPointVec(vec![
            dvec3(-100.0, -100.0, 0.0),
            dvec3(-100.0, 100.0, 0.0),
            dvec3(100.0, 100.0, 0.0),
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
        // Bezier-aware AABB center: ((-1+4)/2, (-1.25+4)/2, 0) = (1.5, 1.375, 0)
        assert_eq!(
            AabbPoint(DVec3::ZERO).locate(&scale_origin),
            dvec3(1.5, 1.375, 0.0)
        );
        scale_origin.with_origin(AabbPoint::CENTER, |x| {
            x.scale(DVec3::splat(3.0));
        });

        let ans = VPointVec(vec![
            dvec3(-6.0, -5.75, 0.0),
            dvec3(3.0, -8.75, 0.0),
            dvec3(-1.5, 0.25, 0.0),
            dvec3(-12.0, 6.25, 0.0),
            dvec3(9.0, 9.25, 0.0),
        ]);
        assert_eq!(scale_origin, ans);
    }
}
