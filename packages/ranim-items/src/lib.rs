use derive_more::{Deref, DerefMut};
use ranim_core::{Extract, traits::{Alignable, Interpolatable, Opacity}, utils::resize_preserving_order_with_repeated_indices};

/// The vectorized item.
pub mod vitem;

/// A Group of type `T`.
///
/// Just like a [`Vec`]
#[derive(Debug, Default, Clone, PartialEq, Deref, DerefMut)]
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

impl<T: Opacity + Alignable + Clone> Alignable for Group<T> {
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

impl<E: Extract> Extract for Group<E> {
    type Target = E::Target;
    fn extract(&self) -> Vec<Self::Target> {
        self.iter().flat_map(|x| x.extract()).collect()
    }
}
