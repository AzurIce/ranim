use glam::DVec3;

use crate::anchor::{Aabb, AabbPoint, Locate};

/// Shifting operations.
///
/// This trait is automatically implemented for [`DVec3`] and `[T]` where `T: Shift`.
pub trait ShiftImpl {
    /// Shift the item by a given vector.
    fn shift(&mut self, offset: DVec3) -> &mut Self;
}

impl ShiftImpl for DVec3 {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        *self += shift;
        self
    }
}

impl<T: Shift> ShiftImpl for [T] {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.shift(shift);
        });
        self
    }
}

/// A trait for shifting operations.
///
/// To implement this trait, you need to implement the [`BoundingBox`] trait first.
pub trait Shift: ShiftImpl {
    /// Put anchor at a given point.
    ///
    /// See [`Anchor`] for more details.
    fn move_anchor_to<A>(&mut self, anchor: A, point: DVec3) -> &mut Self
    where
        A: Locate<Self>,
    {
        self.shift(point - anchor.locate(self));
        self
    }
    /// Put pivot at a given point.
    fn move_to(&mut self, point: DVec3) -> &mut Self
    where
        AabbPoint: Locate<Self>,
    {
        self.move_anchor_to(AabbPoint::CENTER, point)
    }
    /// Put negative anchor of self on anchor of target
    fn move_next_to<T: Aabb + ?Sized>(&mut self, target: &T, anchor: AabbPoint) -> &mut Self
    where
        AabbPoint: Locate<Self>,
    {
        self.move_next_to_padded(target, anchor, 0.0)
    }
    /// Put negative anchor of self on anchor of target, with a distance of `padding`
    fn move_next_to_padded<T: Aabb + ?Sized>(
        &mut self,
        target: &T,
        anchor: AabbPoint,
        padding: f64,
    ) -> &mut Self
    where
        AabbPoint: Locate<Self>,
    {
        let neg_anchor = AabbPoint(-anchor.0);
        self.move_anchor_to(
            neg_anchor,
            Locate::<T>::locate(&anchor, target) + anchor.0.normalize() * padding,
        )
    }
}

impl<T: ShiftImpl + ?Sized> Shift for T {}
