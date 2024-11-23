use crate::rabject::{Rabject, RabjectWithId};

pub mod vmobject;

pub trait Blueprint<T: Rabject> {
    fn build(self) -> RabjectWithId<T>;
}