use crate::render::primitives::Primitive;

pub mod vitem;
pub mod svg_item;

pub trait Entity: Clone {
    type ExtractData;
    type Primitive: Primitive<Entity = Self>;

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
