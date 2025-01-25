use crate::render::primitives::Primitive;

pub mod vitem;

pub trait Entity {
    type ExtractData;
    type Primitive: Primitive<Data = Self::ExtractData>;

    fn extract(&self) -> Option<Self::ExtractData>;
}

/// Blueprints are the data structures that are used to create [`Rabject`]s
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
