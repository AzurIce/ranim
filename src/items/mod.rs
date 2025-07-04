use std::{ops::Deref, sync::atomic::AtomicUsize, vec};

use derive_more::{Deref, DerefMut};
// use variadics_please::all_tuples;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::render::primitives::{Extract, Renderable};

pub mod camera_frame;
pub mod vitem;

static ITEM_CNT: AtomicUsize = AtomicUsize::new(0);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Deref)]
pub struct Id(pub usize);

impl Id {
    pub(crate) fn alloc() -> Self {
        Self(ITEM_CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct ItemId<T> {
    id: Id,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Clone for ItemId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ItemId<T> {}

impl<T> Deref for ItemId<T> {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        self.id.deref()
    }
}

impl<T> ItemId<T> {
    pub(crate) fn new(id: Id) -> Self {
        Self {
            id,
            _phantom: std::marker::PhantomData,
        }
    }
    pub(crate) fn alloc() -> Self {
        Self::new(Id::alloc())
    }
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
