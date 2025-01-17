pub mod canvas;
mod entity;
mod store;

pub use entity::*;
pub use store::*;

use std::ops::{Deref, DerefMut};

use crate::{context::RanimContext, render::Renderer};

#[allow(unused)]
use log::{debug, error, info, trace};

#[allow(unused_imports)]
use std::time::Instant;

pub struct World {
    pub(crate) entities: EntitiesStore<Renderer>,
}

impl Deref for World {
    type Target = EntitiesStore<Renderer>;
    fn deref(&self) -> &Self::Target {
        &self.entities
    }
}

impl DerefMut for World {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entities
    }
}

// Core phases
impl World {
    pub fn tick(&mut self, dt: f32) {
        // info!("[Scene]: TICK STAGE START");
        // let t = Instant::now();
        for (_, entity) in self.entities.iter_mut() {
            entity.tick(dt);
        }
        // info!("[Scene]: TICK STAGE END, took {:?}", t.elapsed());
    }

    pub fn extract(&mut self) {
        // info!("[Scene]: EXTRACT STAGE START");
        // let t = Instant::now();
        for (_, entity) in self.entities.iter_mut() {
            entity.extract();
        }
        // info!("[Scene]: EXTRACT STAGE END, took {:?}", t.elapsed());
    }

    pub fn prepare(&mut self, ctx: &RanimContext) {
        // info!("[Scene]: PREPARE STAGE START");
        // let t = Instant::now();
        for (_, entity) in self.entities.iter_mut() {
            entity.prepare(ctx);
        }
        // info!("[Scene]: PREPARE STAGE END, took {:?}", t.elapsed());
    }

    // pub fn render(&mut self) {
    //     // info!("[Scene]: RENDER STAGE START");
    //     // let t = Instant::now();
    //     self.camera.render(&mut self.ctx, &mut self.entities);
    //     // info!("[Scene]: RENDER STAGE END, took {:?}", t.elapsed());
    // }
}

impl World {
    pub(crate) fn new() -> Self {
        Self {
            entities: EntitiesStore::default(),
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
