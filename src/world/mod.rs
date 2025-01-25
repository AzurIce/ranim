// pub mod canvas;
mod entity;
mod store;

pub use entity::*;
pub use store::*;

use crate::{
    context::RanimContext,
    items::{vitem::VItem, Entity},
};

#[allow(unused)]
use log::{debug, error, info, trace};

#[allow(unused_imports)]
use std::time::Instant;

pub struct World {
    pub(crate) entity_stores: EntityStores,
}

// Core phases
impl World {
    pub fn tick(&mut self, dt: f32) {
        // info!("[Scene]: TICK STAGE START");
        // let t = Instant::now();
        for (_, entity) in self.entity_stores.entry_or_default::<VItem>().iter_mut() {
            entity.tick(dt);
        }
        // info!("[Scene]: TICK STAGE END, took {:?}", t.elapsed());
    }

    pub fn extract(&mut self) {
        // info!("[Scene]: EXTRACT STAGE START");
        // let t = Instant::now();
        for (_, entity) in self.entity_stores.entry_or_default::<VItem>().iter_mut() {
            entity.extract();
        }
        // info!("[Scene]: EXTRACT STAGE END, took {:?}", t.elapsed());
    }

    pub fn prepare(&mut self, ctx: &RanimContext) {
        // info!("[Scene]: PREPARE STAGE START");
        // let t = Instant::now();
        for (_, entity) in self.entity_stores.entry_or_default::<VItem>().iter_mut() {
            entity.prepare(ctx);
        }
        // info!("[Scene]: PREPARE STAGE END, took {:?}", t.elapsed());
    }
}

impl<T: Entity + 'static> Store<T> for World {
    fn get(&self, id: &EntityId<T>) -> &EntityCell<T> {
        // If you have an EntityId of type T, then you must have been insert it into the store
        self.entity_stores.get_store::<T>().unwrap().get(id)
    }
    fn get_mut(&mut self, id: &EntityId<T>) -> &mut EntityCell<T> {
        self.entity_stores.get_store_mut::<T>().unwrap().get_mut(id)
    }
    fn insert(&mut self, entity: T) -> EntityId<T> {
        self.entity_stores.entry_or_default::<T>().insert(entity)
    }
    fn remove(&mut self, id: EntityId<T>) {
        self.entity_stores.get_store_mut::<T>().unwrap().remove(id)
    }
}

impl World {
    pub(crate) fn new() -> Self {
        Self {
            entity_stores: EntityStores::default(),
        }
    }

    // /// Keep the scene static for a given duration
    // ///
    // /// this method writes frames
    // pub fn wait(&mut self, duration: Duration) {
    //     let dt = self.tick_duration().as_secs_f32();
    //     let frames = (duration.as_secs_f32() / dt).ceil() as usize;

    //     for _ in 0..frames {
    //         let start = Instant::now();
    //         self.update_frame(false);
    //         trace!(
    //             "[Scene/wait] one complete frame(update_frame) cost: {:?}",
    //             start.elapsed()
    //         );
    //     }
    // }
}
