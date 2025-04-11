use glam::DVec3;
use group::Group;
use ranim_macros::item;
use vitem::{Circle, Line, VItem};

use crate::{
    RanimTimeline,
    animation::{AnimSchedule, AnimationSpan},
    timeline::{ItemMark, TimelineItem},
    traits::Empty,
};

pub mod camera_frame;
pub mod group;
pub mod svg_item;
pub mod vitem;

impl<'r, 't: 'r, T> Group<Rabject<'t, T>> {
    pub fn lagged_anim(
        &'r mut self,
        lag_ratio: f64,
        anim_builder: impl FnOnce(&'r mut Rabject<'t, T>) -> AnimSchedule<'r, 't, T> + Clone,
    ) -> Group<AnimSchedule<'r, 't, T>> {
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

// #[item]
pub struct Arrow {
    tip: VItem,
    line: VItem,
}

impl Arrow {
    pub fn new() -> Self {
        Self {
            tip: Circle(1.0).build(),
            line: Line(0.2 * DVec3::NEG_Y, 0.2 * DVec3::Y).build(),
        }
    }
}

impl<'t> TimelineItem<'t, ItemMark> for Arrow {
    type Inserted = ArrowRabject<'t>;
    fn insert_into_timeline(self, timeline: &'t RanimTimeline) -> Self::Inserted {
        ArrowRabject {
            tip: RanimTimeline::insert(timeline, self.tip),
            line: RanimTimeline::insert(timeline, self.line),
        }
    }
}

impl<'t> ArrowRabject<'t> {
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Rabject<'t, VItem>> {
        let mut_parts = self.mut_parts();
        [mut_parts.tip, mut_parts.line].into_iter()
    }
}

pub trait MutParts<'a> {
    type Owned;
    type Mut: 'a;
    fn mut_parts(&'a mut self) -> Self::Mut;
    fn owned(&'a self) -> Self::Owned;
}

pub trait ArrowMethods<'a>: MutParts<'a, Mut = ArrowMutParts<'a>> {
    fn set_tip(&'a mut self, tip: VItem);
    fn set_line(&'a mut self, line: VItem);
}

impl<'a, T: MutParts<'a, Mut = ArrowMutParts<'a>>> ArrowMethods<'a> for T {
    fn set_tip(&'a mut self, tip: VItem) {
        *self.mut_parts().tip = tip;
    }

    fn set_line(&'a mut self, line: VItem) {
        *self.mut_parts().line = line;
    }
}

// Example usage:
fn foo() {
    let timeline = RanimTimeline::new();

    let mut arrow = Arrow {
        tip: VItem::empty(),
        line: VItem::empty(),
    };

    arrow.set_tip(VItem::empty());

    let mut arrow_rabject = ArrowRabject {
        tip: Rabject {
            timeline: &timeline,
            id: 0,
            data: arrow.tip,
        },
        line: Rabject {
            timeline: &timeline,
            id: 1,
            data: arrow.line,
        },
    };

    arrow_rabject.set_tip(VItem::empty());
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
