pub mod animation;
pub mod color;
pub mod components;
pub mod traits;
pub mod utils;

pub mod primitives {
    pub mod vitem;
    pub mod camera_frame;
}

use derive_more::{Deref, DerefMut};
pub use glam;

use crate::{traits::{Alignable, Interpolatable, Opacity}, utils::{resize_preserving_order, resize_preserving_order_with_repeated_indices}};

pub mod prelude {
    pub use crate::color::prelude::*;
    pub use crate::traits::*;
}

/// Extract a [`Extract::Target`] from reference.
pub trait Extract {
    /// The extraction result
    type Target;
    /// Extract a [`Extract::Target`] from reference.
    fn extract(&self) -> Self::Target;
}

impl<E: Extract> Extract for Group<E> {
    type Target = Vec<E::Target>;
    fn extract(&self) -> Self::Target {
        self.iter().map(|x| x.extract()).collect()
    }
}

/// A Group of type `T`.
///
/// Just like a [`Vec`]
#[derive(Debug, Default, Clone, Deref, DerefMut)]
pub struct Group<T>(pub Vec<T>);

impl<T> IntoIterator for Group<T> {
    type IntoIter = std::vec::IntoIter<T>;
    type Item = T;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Group<T> {
    type IntoIter = std::slice::Iter<'a, T>;
    type Item = &'a T;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Group<T> {
    type IntoIter = std::slice::IterMut<'a, T>;
    type Item = &'a mut T;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<T> FromIterator<T> for Group<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(Vec::from_iter(iter))
    }
}


impl<T: Interpolatable> Interpolatable for Group<T> {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        self.into_iter()
            .zip(target)
            .map(|(a, b)| a.lerp(b, t))
            .collect()
    }
}

impl<T: Alignable + Clone> Alignable for Group<T> {
    fn is_aligned(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a.is_aligned(b))
    }
    fn align_with(&mut self, other: &mut Self) {
        let len = self.len().max(other.len());
        if self.len() != len {
            resize_preserving_order(&self.0, len);
        }
        if other.len() != len {
            resize_preserving_order(&other.0, len);
        }
        self.iter_mut()
            .zip(other)
            .for_each(|(a, b)| a.align_with(b));
    }
}

/// A Group of type `T`, but aligns by transparent repeated items.
///
/// Just like a [`Vec`]
#[derive(Debug, Default, Clone, Deref, DerefMut)]
pub struct GroupWithOpacity<T: Opacity>(pub Vec<T>);

impl<T: Opacity> IntoIterator for GroupWithOpacity<T> {
    type IntoIter = std::vec::IntoIter<T>;
    type Item = T;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T: Opacity> IntoIterator for &'a GroupWithOpacity<T> {
    type IntoIter = std::slice::Iter<'a, T>;
    type Item = &'a T;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, T: Opacity> IntoIterator for &'a mut GroupWithOpacity<T> {
    type IntoIter = std::slice::IterMut<'a, T>;
    type Item = &'a mut T;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<T: Opacity> FromIterator<T> for GroupWithOpacity<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(Vec::from_iter(iter))
    }
}


impl<T: Interpolatable + Opacity> Interpolatable for GroupWithOpacity<T> {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        self.into_iter()
            .zip(target)
            .map(|(a, b)| a.lerp(b, t))
            .collect()
    }
}

impl<T: Alignable + Clone + Opacity> Alignable for GroupWithOpacity<T> {
    fn is_aligned(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a.is_aligned(b))
    }
    fn align_with(&mut self, other: &mut Self) {
        let len = self.len().max(other.len());

        let transparent_repeated = |items: &mut Vec<T>, repeat_idxs: Vec<usize>| {
            for idx in repeat_idxs {
                items[idx].set_opacity(0.0);
            }
        };
        if self.len() != len {
            let (mut items, idxs) = resize_preserving_order_with_repeated_indices(&self.0, len);
            transparent_repeated(&mut items, idxs);
            self.0 = items;
        }
        if other.len() != len {
            let (mut items, idxs) = resize_preserving_order_with_repeated_indices(&other.0, len);
            transparent_repeated(&mut items, idxs);
            other.0 = items;
        }
        self.iter_mut()
            .zip(other)
            .for_each(|(a, b)| a.align_with(b));
    }
}
