use group::Group;

use crate::animation::{AnimSchedule, AnimationSpan};

pub mod camera_frame;
pub mod group;
pub mod svg_item;
pub mod vitem;

impl<'r, T> Group<Rabject<T>> {
    pub fn lagged_anim(
        &'r mut self,
        lag_ratio: f64,
        anim_builder: impl FnOnce(&'r mut Rabject<T>) -> AnimSchedule<'r, T> + Clone,
    ) -> Group<AnimSchedule<'r, T>> {
        let n = self.as_ref().len();

        let mut anim_schedules = self
            .as_mut()
            .iter_mut()
            .map(|rabject| (anim_builder.clone())(rabject))
            .collect::<Group<_>>();

        let duration = anim_schedules[0].anim.duration_secs;
        let lag_time = duration * lag_ratio;
        anim_schedules
            .iter_mut()
            .enumerate()
            .for_each(|(i, schedule)| {
                schedule.anim.padding = (i as f64 * lag_time, (n - i - 1) as f64 * lag_time);
                // println!("{} {:?} {}", schedule.anim.span_len(), schedule.anim.padding, schedule.anim.duration_secs);
            });
        anim_schedules
    }
}

/// An `Rabject` is a wrapper of an entity that can be rendered.
///
/// The `Rabject`s with same `Id` will use the same `EntityTimeline` to animate.
pub struct Rabject<T> {
    pub id: usize,
    pub data: T,
}

impl<T: 'static> Rabject<T> {
    pub fn schedule<'r>(
        &'r mut self,
        anim_builder: impl FnOnce(&mut Self) -> AnimationSpan<T>,
    ) -> AnimSchedule<'r, T> {
        let animation = (anim_builder)(self);
        AnimSchedule::new(self, animation)
    }
}

// MARK: Entity

// /// A renderable entity in ranim
// ///
// /// You can implement your own entity by implementing this trait.
// ///
// /// In Ranim, every item `T` is just plain data. After [`RanimTimeline::insert`]ed to [`RanimTimeline`],
// /// the item will have an id and its corresponding [`crate::timeline::RabjectTimeline`].
// ///
// /// The resources (buffer, texture, etc) rendering an item needs are called **RenderInstance**,
// /// and all of them are managed by ranim outside of timeline in a struct [`RenderInstances`].
// ///
// /// The [`RenderInstances`] is basically a store of [`RenderInstance`]s based on [`std::collections::HashMap`].
// /// - The key is the combination of [`Rabject::id`] and [`RenderInstance`]'s [`std::any::TypeId`]
// /// - The value is the [`RenderInstance`]
// ///
// /// For now, there are two types of [`RenderInstance`]:
// /// - [`crate::render::primitives::vitem::VItemPrimitive`]: The core primitive to render vectorized items.
// /// - [`crate::render::primitives::svg_item::SvgItemPrimitive`]
// ///
// /// You can check the builtin implementations of [`Entity`] for mor details.
// ///
/// Blueprints are the data structures that are used to create an Item
pub trait Blueprint<T> {
    fn build(self) -> T;
}

pub trait Updatable {
    fn update_from(&mut self, other: &Self);
}

impl<T: Clone> Updatable for T {
    fn update_from(&mut self, other: &Self) {
        *self = other.clone();
    }
}
