use std::sync::atomic::AtomicUsize;

use derive_more::{Deref, DerefMut};
use group::Group;
// use variadics_please::all_tuples;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    animation::{AnimSchedule, AnimationSpan},
    render::primitives::{Extract, Renderable},
};

pub mod camera_frame;
pub mod group;
pub mod vitem;

impl<T> Group<T> {
    pub fn lagged_anim(
        self,
        lag_ratio: f64,
        anim_builder: impl FnOnce(T) -> AnimationSpan<T> + Clone,
    ) -> Group<AnimationSpan<T>> {
        let n = self.as_ref().len();

        let mut anim_schedules = self
            .into_iter()
            .map(|rabject| (anim_builder.clone())(rabject))
            .collect::<Group<_>>();

        let duration = anim_schedules[0].duration_secs;
        let lag_time = duration * lag_ratio;
        anim_schedules
            .iter_mut()
            .enumerate()
            .for_each(|(i, anim)| {
                anim.padding = (i as f64 * lag_time, (n - i - 1) as f64 * lag_time);
                // println!("{} {:?} {}", schedule.anim.span_len(), schedule.anim.padding, schedule.anim.duration_secs);
            });
        anim_schedules
    }
}

static RABJECT_CNT: AtomicUsize = AtomicUsize::new(0);

/// An `Rabject` is a wrapper of an entity that can be rendered.
///
/// The `Rabject`s with same `Id` will use the same `EntityTimeline` to animate.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Deref, DerefMut)]
pub struct PinnedItem<T> {
    id: usize,
    #[deref]
    #[deref_mut]
    pub data: T,
}

impl<T> PinnedItem<T> {
    pub fn id(&self) -> usize {
        self.id
    }
    pub(crate) fn new(data: T) -> Self {
        Self {
            id: RABJECT_CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            data,
        }
    }
}

// impl<T: 'static> Rabject<T> {
//     pub fn schedule(
//         &mut self,
//         anim_builder: impl FnOnce(&mut Self) -> AnimationSpan<T>,
//     ) -> AnimSchedule<T> {
//         let animation = (anim_builder)(self);
//         AnimSchedule::new(self, animation)
//     }
// }

/// Blueprints are the data structures that are used to create an Item
#[deprecated(
    since = "0.1.0-alpha.14",
    note = "Use the refactored item system instead"
)]
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

impl<T: Extract<Target = Target>, Target: Renderable + 'static> VisualItem for T {
    fn extract_renderable(&self) -> Box<dyn Renderable> {
        Box::new(Extract::extract(self))
    }
}

/// The item which can be extracted into a [`Renderable`]
///
/// VisualItem is one kind of [`crate::timeline::RanimItem`].
/// This is automatically implemented for the types that [`Extract`] to a [`Renderable`].
pub trait VisualItem {
    fn extract_renderable(&self) -> Box<dyn Renderable>;
}

// TODO: This causes some clone
// The render is also based on the decomposed result?
// pub trait Item {
//     type Target;
//     fn decompose(&self) -> Self::Target;
// }

// macro_rules! impl_into_group {
//     ($($T:ident),*) => {
//         impl<$($T: Into<I>),*,I: BaseItem> From<($($T,)*)> for Group<I> {
//             fn from(value: ($($T,)*)) -> Self {
//                 #[allow(non_snake_case, reason = "`all_tuples!()` generates non-snake-case variable names.")]
//                 let ($($T,)*) = value;
//                 Self(vec![$($T.into()),*])
//             }
//         }
//     };
// }

// all_tuples!(impl_into_group, 1, 16, T);

// impl<T: Into<E>, E: RenderableItem> RenderableItem for T {
//     fn prepare_for_id(
//         &self,
//         ctx: &crate::context::WgpuContext,
//         render_instances: &mut crate::render::primitives::RenderInstances,
//         id: usize,
//     ) {

//     }
//     fn renderable_of_id<'a>(
//         &'a self,
//         render_instances: &'a crate::render::primitives::RenderInstances,
//         id: usize,
//     ) -> Option<&'a dyn crate::render::primitives::Renderable> {
//     }
// }

#[cfg(test)]
mod test {
    fn id<T>(x: &T) -> usize {
        x as *const T as usize
    }

    fn foo_move<T>(x: T) {
        println!("x: {}", id(&x));
    }

    #[test]
    fn foo() {
        let mut a = 12;
        println!("a: {}", id(&a));
        // a = 13;
        // println!("assigned a: {}", id(&a));
        // let b = a;
        // println!("b: {}", id(&b));
        foo_move(a);

        let mut a = String::from("hello");
        println!("a: {}", id(&a));
        // a = String::from("world");
        // println!("assigned a: {}", id(&a));
        // a.clear();
        // println!("update a: {}", id(&a));
        // let b = a;
        // println!("b: {}", id(&b));
        foo_move(a);

    }
}