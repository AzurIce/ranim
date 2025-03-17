use std::ops::{Deref, DerefMut};

/// A group of things.
///
/// The inner of a group is a [`Vec`], it has the ownership of the elements.
///
/// [`Group<T>`] implements [`FromIterator`], [`IntoIterator<Item = T>`], so it can be
/// collected from an iterator, and also can be used for `impl IntoIterator<Item = T>`:
///
/// ```rust
/// let group = (0..9).map(|i| Square(100.0 * i as f32).build()).collect::<Group<_>>();
/// let group = group.into_iter().map(|item| timeline.insert(item)).collect::<Group<_>>();
/// ```
///
/// For a group of items, you can use [`Group::lagged_anim`] to create animation on every item:
///
/// ```rust
/// timeline.play(group.lagged_anim(0.2, |item| {
///     item.write()
/// }).with_duration(5.0).apply());
/// ```
/// 
/// For some animation (like [`crate::animation::transform::Transform`]), it may support
/// creating directly for item's slice. This often happens when some operation on the group
/// is not equivalent to applying the same operation on each item (like [`crate::components::Transformable::scale`]).
/// 
/// For example, if logo is a `Group<VItem>` with six elements:
/// 
/// ```rust
/// let scale = [
///     vec3(scale, 1.0, 1.0),
///     vec3(scale, scale, 1.0),
///     vec3(scale, scale, 1.0),
/// ];
/// let anchor = [
///     Anchor::edge(-1, 0, 0),
///     Anchor::edge(1, 1, 0),
///     Anchor::edge(1, -1, 0),
/// ];
/// logo.chunks_mut(2)
///     .zip(scale.into_iter().zip(anchor))
///     .for_each(|(chunk, (scale, anchor))| {
///         timeline.play(
///             chunk
///                 .transform(|data| {
///                     data.scale_by_anchor(scale, anchor)
///                         .scale_by_anchor(vec3(0.9, 0.9, 1.0), Anchor::origin())
///                         .shift(vec3(0.0, frame_size.y / 9.0, 0.0));
///                 })
///                 .with_rate_func(smooth)
///                 .apply(),
///         );
///     });
/// ```
///
/// [`AsRef<[T]>`](AsRef) and [`AsMut<[T]>`](AsMut) are implemented for `Group<T>`.
///
///
#[derive(Clone)]
pub struct Group<T>(pub Vec<T>);

impl<T> Group<T> {
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.0.iter_mut()
    }
}

impl<T> Deref for Group<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Group<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> AsRef<[T]> for Group<T> {
    fn as_ref(&self) -> &[T] {
        &self.0
    }
}

impl<T> AsMut<[T]> for Group<T> {
    fn as_mut(&mut self) -> &mut [T] {
        &mut self.0
    }
}

impl<T> FromIterator<T> for Group<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T> IntoIterator for Group<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
