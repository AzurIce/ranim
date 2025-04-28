use std::ops::{Deref, DerefMut};

use glam::DVec3;
use log::warn;

use crate::{
    components::Anchor,
    traits::{BoundingBox, Fill, Position, Stroke},
};

/// A group of things.
///
/// The inner of a group is a [`Vec`], it has the ownership of the elements.
///
/// You can construct a group directly from an [`Vec`]:
///
/// ```rust
/// let group = Group(vec![1, 2, 3, 4]);
/// ```
///
/// # Group of items
///
/// For convinience, [`Group<T>`] also implements [`FromIterator`] and
/// [`IntoIterator<Item = T>`], so it can be collected from an iterator,
/// and also can be used for `impl IntoIterator<Item = T>`.
///
/// ```rust
/// let mut group = (0..9).map(|i| Square(100.0 * i as f32).build()).collect::<Group<_>>();
/// ```
///
/// Note that some operations like [`Position`] for a group of items
/// should have different implementation for a single item and a group of items.
/// (For example, scaling the whole group is not equivalent to scaling each item).
///
/// In this case, `[T]` normally has the different implementation. And [`Group<T>`]
/// implements [`Deref`], [`DerefMut`], [`AsRef`], [`AsMut`] to `[T]`.
/// So a group of items (or its slice) will have the implementation:
///
/// ```rust
/// group[0..3].scale_by_anchor(vec3(0.5, 0.5, 1.0), Anchor::origin());
/// ```
///
/// A group of items can be inserted into the timeline with
/// [`crate::timeline::RanimTimeline::insert`] to get
/// a group of [`crate::items::Rabject`]s:
///
/// ```rust
/// let group = timeline.insert(group); // returns `Group<Rabject<T>>`
/// ```
///
/// # Group of [`crate::items::Rabject`]s
///
/// You can use [`Group::lagged_anim`] to create animation on every item:
///
/// ```rust
/// timeline.play(group.lagged_anim(0.2, |item| {
///     item.write()
/// }).with_duration(5.0).apply());
/// ```
///
/// For some animations (like [`crate::animation::transform::Transform`]), it may support
/// creating directly from item's slice. Since it may involves some group operations which
/// is not equivalent to applying the same operation on each item (like [`Position`]).
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
#[derive(Clone, Default)]
pub struct Group<T>(pub Vec<T>);

impl<T> Group<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.0.iter_mut()
    }

    pub fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }

    pub fn push(&mut self, item: T) {
        self.0.push(item);
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

impl<T: BoundingBox> BoundingBox for [T] {
    fn get_bounding_box(&self) -> [DVec3; 3] {
        let [min, max] = self
            .iter()
            .map(|x| x.get_bounding_box())
            .map(|[min, _, max]| [min, max])
            .reduce(|[acc_min, acc_max], [min, max]| [acc_min.min(min), acc_max.max(max)])
            .unwrap_or([DVec3::ZERO, DVec3::ZERO]);
        if min == max {
            warn!("Empty bounding box, is the slice empty?")
        }
        [min, (min + max) / 2.0, max]
    }
}

impl<T: Position> Position for [T] {
    fn shift(&mut self, shift: DVec3) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.shift(shift);
        });
        self
    }
    fn rotate_by_anchor(&mut self, angle: f64, axis: DVec3, anchor: Anchor) -> &mut Self {
        let anchor = match anchor {
            Anchor::Point(p) => p,
            Anchor::Edge(e) => self.get_bounding_box_point(e),
        };
        self.iter_mut().for_each(|x| {
            x.rotate_by_anchor(angle, axis, Anchor::Point(anchor));
        });
        self
    }
    fn scale_by_anchor(&mut self, scale: DVec3, anchor: Anchor) -> &mut Self {
        let anchor = match anchor {
            Anchor::Point(p) => p,
            Anchor::Edge(e) => self.get_bounding_box_point(e),
        };
        self.iter_mut().for_each(|x| {
            x.scale_by_anchor(scale, Anchor::Point(anchor));
        });
        self
    }
}

impl<T: Stroke> Stroke for [T] {
    fn apply_stroke_func(&mut self, f: impl for<'a> Fn(&'a mut [crate::components::width::Width])) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.apply_stroke_func(&f);
        });
        self
    }
    fn set_stroke_color(&mut self, color: color::AlphaColor<color::Srgb>) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.set_stroke_color(color);
        });
        self
    }
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.set_stroke_opacity(opacity);
        });
        self
    }
}

impl<T: Fill> Fill for [T] {
    fn fill_color(&self) -> color::AlphaColor<color::Srgb> {
        self[0].fill_color()
    }
    fn set_fill_color(&mut self, color: color::AlphaColor<color::Srgb>) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.set_fill_color(color);
        });
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.iter_mut().for_each(|x| {
            x.set_fill_opacity(opacity);
        });
        self
    }
}
