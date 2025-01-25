use std::{fmt::Debug, marker::PhantomData, ops::Deref};

use crate::utils::Id;

pub struct EntityId<E>(Id, PhantomData<E>);

impl<E> Debug for EntityId<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EntityId({:?})", self.0)
    }
}

impl<E> EntityId<E> {
    pub fn from_id(id: Id) -> Self {
        Self(id, PhantomData)
    }
}

impl<E> Deref for EntityId<E> {
    type Target = Id;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
