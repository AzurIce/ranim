use glam::Vec2;

use crate::{
    prelude::Empty,
    render::{primitives::Extract, CameraFrame},
};

pub mod svg_item;
pub mod vitem;

pub trait Entity: Clone + Empty {
    type Primitive: Extract<Self> + Default;

    #[allow(unused)]
    fn clip_box(&self, camera: &CameraFrame) -> [Vec2; 4] {
        [
            Vec2::new(-1.0, -1.0),
            Vec2::new(-1.0, 1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
        ]
    }
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
