use std::{fmt::Debug, ops::Deref};

pub use ranim_core::Group;
pub use ranim_items::*;

/// An item id.
pub struct ItemId<T> {
    id: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Debug for ItemId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ItemId")
            .field("id", &self.id)
            .field("type", &std::any::type_name::<T>())
            .finish()
    }
}

impl<T> Deref for ItemId<T> {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl<T> ItemId<T> {
    /// Get the inner id.
    pub fn inner(&self) -> usize {
        self.id
    }
    pub(crate) fn new(id: usize) -> Self {
        Self {
            id,
            _phantom: std::marker::PhantomData,
        }
    }
}
