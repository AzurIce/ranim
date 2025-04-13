use camera_frame::CameraFrame;
use group::Group;

use crate::{
    RanimTimeline,
    animation::{AnimSchedule, AnimationSpan},
    render::primitives::{Extract, Primitive, Renderable},
    timeline::{RabjectTimeline, Timeline},
};

pub mod camera_frame;
pub mod group;
pub mod svg_item;
pub mod vitem;

pub trait LaggedAnim<'r, 't, T> {
    fn lagged_anim(
        &'r mut self,
        lag_ratio: f64,
        anim_builder: impl FnOnce(&'r mut Rabject<'t, T>) -> AnimSchedule<'r, 't, T> + Clone,
    ) -> Group<AnimSchedule<'r, 't, T>>;
}

impl<'r, 't: 'r, T: 'r, R> LaggedAnim<'r, 't, T> for R
where
    R: IterMutRabjects<'t, 'r, T> + ?Sized,
{
    fn lagged_anim(
        &'r mut self,
        lag_ratio: f64,
        anim_builder: impl FnOnce(&'r mut Rabject<'t, T>) -> AnimSchedule<'r, 't, T> + Clone,
    ) -> Group<AnimSchedule<'r, 't, T>> {
        let iter = self.iter_mut();

        let mut anim_schedules = iter
            .map(|rabject| (anim_builder.clone())(rabject))
            .collect::<Group<_>>();
        let n = anim_schedules.len();

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

/// A marker trait for auto implementation of [`MutParts`]
pub trait BaseMutParts: Clone {}

impl<'a, T: BaseMutParts + 'a> MutParts<'a> for T {
    type Owned = T;
    type Mut = &'a mut T;
    fn mut_parts(&'a mut self) -> Self::Mut {
        self
    }
    fn owned(&'a self) -> Self::Owned {
        self.clone()
    }
}

impl<'a, T: BaseMutParts + 'a> MutParts<'a> for Rabject<'_, T> {
    type Owned = T;
    type Mut = &'a mut T;
    fn mut_parts(&'a mut self) -> Self::Mut {
        &mut self.data
    }
    fn owned(&'a self) -> Self::Owned {
        self.data.clone()
    }
}

impl<'a, 't, T: BaseMutParts + 'a> IntoIterator for &'a mut Rabject<'t, T> {
    type Item = &'a mut Rabject<'t, T>;
    type IntoIter = std::iter::Once<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}

pub trait MutParts<'a> {
    type Owned;
    type Mut: 'a;
    fn mut_parts(&'a mut self) -> Self::Mut;
    fn owned(&'a self) -> Self::Owned;
}

// MARK: Item
/// What can be inserted into the timeline
///
/// [`Item::BaseItem`] is the type of the underlying [`RabjectTimeline`]
/// [`Item::Rabject`] is the Rabject type that will be returned
pub trait Item {
    type BaseItem;
    type Rabject<'t>;
    fn insert_into_timeline(self, ranim_timeline: &RanimTimeline) -> Self::Rabject<'_>;
}

// impl for RenderableItems
impl<T, P> Item for T
where
    T: Extract<Primitive = P> + Clone + 'static,
    P: Renderable + Primitive + 'static,
{
    type BaseItem = T;
    type Rabject<'t> = Rabject<'t, T>;
    fn insert_into_timeline(self, ranim_timeline: &RanimTimeline) -> Self::Rabject<'_> {
        let timeline = RabjectTimeline::new(self.clone());
        let timeline = Timeline::RenderableItem(Box::new(timeline));
        Rabject {
            id: ranim_timeline.insert_timeline(timeline),
            data: self,
            timeline: ranim_timeline,
        }
    }
}

// imple for CameraFrame
impl Item for CameraFrame {
    type BaseItem = CameraFrame;
    type Rabject<'t> = Rabject<'t, CameraFrame>;
    fn insert_into_timeline(self, ranim_timeline: &RanimTimeline) -> Self::Rabject<'_> {
        let timeline = RabjectTimeline::new(self.clone());
        let timeline = Timeline::CameraFrame(Box::new(timeline));
        Rabject {
            id: ranim_timeline.insert_timeline(timeline),
            data: self,
            timeline: ranim_timeline,
        }
    }
}

pub trait IterMutRabjects<'t: 'r, 'r, T> {
    fn iter_mut<'a, 'b>(&'a mut self) -> impl Iterator<Item = &'b mut Rabject<'t, T>>
    where
        'a: 'b,
        't: 'b,
        T: 'b;
}

impl<'t: 'r, 'r, T> IterMutRabjects<'t, 'r, T> for [Rabject<'t, T>] {
    fn iter_mut<'a, 'b>(&'a mut self) -> impl Iterator<Item = &'b mut Rabject<'t, T>>
    where
        'a: 'b,
        't: 'b,
        T: 'b,
    {
        self.iter_mut()
    }
}
/// An `Rabject` is a wrapper of an entity that can be rendered.
///
/// The `Rabject`s with same `Id` will use the same `EntityTimeline` to animate.
pub struct Rabject<'t, T> {
    pub timeline: &'t RanimTimeline,
    pub id: usize,
    pub data: T,
}

impl<T> Drop for Rabject<'_, T> {
    fn drop(&mut self) {
        self.timeline.hide(self);
        // TODO: remove it
    }
}

impl<'t, T: 'static> Rabject<'t, T> {
    pub fn schedule<'r>(
        &'r mut self,
        anim_builder: impl FnOnce(&mut Self) -> AnimationSpan<T>,
    ) -> AnimSchedule<'r, 't, T> {
        let animation = (anim_builder)(self);
        AnimSchedule::new(self, animation)
    }
}

impl<'t, T> Rabject<'t, T> {
    fn iter_mut<'a, 'b>(&'a mut self) -> impl Iterator<Item = &'b mut Rabject<'t, T>>
    where
        'a: 'b,
        't: 'b,
        T: 'b,
    {
        std::iter::once(self)
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
