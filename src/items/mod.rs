use std::{sync::atomic::AtomicUsize, vec};

use derive_more::{Deref, DerefMut};
// use variadics_please::all_tuples;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::render::primitives::{Extract, Renderable};

pub mod camera_frame;
pub mod vitem;

static TIMELINE_CNT: AtomicUsize = AtomicUsize::new(0);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct TimelineId<T> {
    id: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> TimelineId<T> {
    pub fn id(&self) -> usize {
        self.id
    }
    pub(crate) fn new() -> Self {
        Self {
            id: TIMELINE_CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            _phantom: std::marker::PhantomData,
        }
    }
}

/// Blueprints are the data structures that are used to create an Item
#[deprecated(
    since = "0.1.0-alpha.14",
    note = "Use the refactored item system instead"
)]
pub trait Blueprint<T> {
    fn build(self) -> T;
}

impl<T: Extract<Target = Target>, Target: Renderable + 'static> VisualItem for T {
    fn extract_renderable(&self) -> Box<dyn Renderable> {
        Box::new(Extract::extract(self))
    }
}

/// The item which can be extracted into a [`Renderable`]
///
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

#[derive(Debug, Default, Clone, Deref, DerefMut)]
pub struct Group<T>(pub Vec<T>);

impl<T> IntoIterator for Group<T> {
    type IntoIter = vec::IntoIter<T>;
    type Item = T;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Group<T> {
    type IntoIter = std::slice::Iter<'a, T>;
    type Item = &'a T;
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Group<T> {
    type IntoIter = std::slice::IterMut<'a, T>;
    type Item = &'a mut T;
    fn into_iter(self) -> Self::IntoIter {
        (&mut self.0).into_iter()
    }
}

impl<T> FromIterator<T> for Group<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(Vec::from_iter(iter))
    }
}