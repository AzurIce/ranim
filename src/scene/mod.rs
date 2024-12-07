pub mod file_writer;
pub mod store;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fs,
    path::Path,
    time::{Duration, Instant},
};

use file_writer::{FileWriter, FileWriterBuilder};
use image::{ImageBuffer, Rgba};

#[allow(unused_imports)]
use log::{debug, info};
use store::{RabjectStore, RabjectStores};

use crate::{
    camera::Camera,
    rabject::{
        vgroup::{VGroup, VGroupPrimitive},
        vmobject::{primitive::VMobjectPrimitive, VMobject},
        Primitive, Rabject, RabjectId,
    },
    updater::Updater,
    utils::Id,
    RanimContext,
};

#[allow(unused)]
use log::trace;
pub struct UpdaterStore<R: Rabject> {
    /// The updater
    pub updater: Box<dyn Updater<R>>,
    /// The id of the target rabject
    pub target_id: RabjectId<R>,
}

pub struct Scene {
    ctx: RanimContext,
    pub camera: Camera,
    /// Rabjects in the scene
    pub rabjects: RabjectStores,
    /// Updaters for the rabjects
    ///
    /// Rabject's type id -> Vec<(Updater's id, Updater<Rabject>)>
    pub updaters: HashMap<TypeId, Box<dyn Any>>,

    pub time: f32,
    pub frame_count: usize,

    /// The writer for the output.mp4 video
    pub video_writer: Option<FileWriter>,
    /// Whether to save the frame to 'output/image-x.png'
    pub save_frame: bool,
}

// Entity management - Low level apis
impl Scene {}

// Entity management - High level apis
impl Scene {
    /// Insert a rabject to the scene
    ///
    /// See [`RabjectStores::insert`]
    pub fn insert<R: Rabject + 'static>(&mut self, rabject: R) -> RabjectId<R> {
        self.rabjects.insert(rabject)
    }

    /// Remove a rabject from the scene
    ///
    /// See [`RabjectStores::remove`]
    pub fn remove<R: Rabject>(&mut self, id: RabjectId<R>) {
        self.rabjects.remove(&id);
    }

    /// Get a reference of a rabject from the scene
    ///
    /// See [`RabjectStores::get`]
    pub fn get<R: Rabject + 'static>(&self, id: RabjectId<R>) -> Option<&R> {
        self.rabjects.get(&id)
    }

    /// Get a mutable reference of a rabject from the scene
    ///
    /// See [`RabjectStores::get_mut`]
    pub fn get_mut<R: Rabject + 'static>(&mut self, id: RabjectId<R>) -> Option<&mut R> {
        self.rabjects.get_mut(&id)
    }
}

// the core phases
impl Scene {
    pub fn tick(&mut self, dt: f32) {
        info!("[Scene]: TICK STAGE START");
        let t = Instant::now();
        self.time += dt;
        self.frame_count += 1;

        self.updaters.iter_mut().for_each(|(_, updaters)| {
            if let Some(updaters) = updaters.downcast_mut::<Vec<(Id, UpdaterStore<VMobject>)>>() {
                updaters.retain_mut(|(_, updater_store)| {
                    self.rabjects
                        .get_mut::<VMobject>(&updater_store.target_id)
                        .map(|rabject| {
                            let keep = updater_store.updater.on_update(rabject, dt);
                            if !keep {
                                updater_store.updater.on_destroy(rabject);
                            }
                            keep
                        })
                        .unwrap_or(false)
                });
            }
        });

        info!("[Scene]: TICK STAGE END, took {:?}", t.elapsed());
    }

    pub fn extract(&mut self) {
        info!("[Scene]: EXTRACT STAGE START");
        let t = Instant::now();
        for (_, entities) in self.rabjects.iter_mut() {
            for (_, entity) in entities.iter_mut() {
                if let Some(rabject_store) = entity.downcast_mut::<RabjectStore<VMobject>>() {
                    rabject_store.render_data = Some(rabject_store.rabject.extract());
                } else if let Some(rabject_store) = entity.downcast_mut::<RabjectStore<VGroup>>() {
                    rabject_store.render_data = Some(rabject_store.rabject.extract());
                }
            }
        }
        info!("[Scene]: EXTRACT STAGE END, took {:?}", t.elapsed());
    }

    pub fn prepare(&mut self) {
        info!("[Scene]: PREPARE STAGE START");
        let t = Instant::now();
        for (_, entities) in self.rabjects.iter_mut() {
            for (_, entity) in entities.iter_mut() {
                if let Some(entity) = entity.downcast_mut::<RabjectStore<VMobject>>() {
                    if let Some(render_resource) = entity.render_resource.as_mut() {
                        render_resource.update(
                            &mut self.ctx.wgpu_ctx,
                            &entity.render_data.as_ref().unwrap(),
                        );
                    } else {
                        entity.render_resource = Some(VMobjectPrimitive::init(
                            &mut self.ctx.wgpu_ctx,
                            &entity.render_data.as_ref().unwrap(),
                        ));
                    }
                } else if let Some(entity) = entity.downcast_mut::<RabjectStore<VGroup>>() {
                    if let Some(render_resource) = entity.render_resource.as_mut() {
                        render_resource.update(
                            &mut self.ctx.wgpu_ctx,
                            &entity.render_data.as_ref().unwrap(),
                        );
                    } else {
                        entity.render_resource = Some(VGroupPrimitive::init(
                            &mut self.ctx.wgpu_ctx,
                            &entity.render_data.as_ref().unwrap(),
                        ));
                    }
                }
            }
        }
        info!("[Scene]: PREPARE STAGE END, took {:?}", t.elapsed());
    }

    pub fn render(&mut self) {
        info!("[Scene]: RENDER STAGE START");
        let t = Instant::now();
        self.camera.update_uniforms(&self.ctx.wgpu_ctx);
        self.camera.clear_screen(&self.ctx.wgpu_ctx);
        self.camera
            .render::<VMobject>(&mut self.ctx, &mut self.rabjects);
        self.camera
            .render::<VGroup>(&mut self.ctx, &mut self.rabjects);
        info!("[Scene]: RENDER STAGE END, took {:?}", t.elapsed());
    }
}

impl Scene {
    pub fn new_with_video_writer_builder(builder: FileWriterBuilder) -> Self {
        let ctx = RanimContext::new();
        let camera = Camera::new(
            &ctx,
            builder.width as usize,
            builder.height as usize,
            builder.fps,
        );
        let video_writer = builder.build();
        Self {
            ctx,
            camera,
            rabjects: RabjectStores::default(),
            updaters: HashMap::new(),
            time: 0.0,
            frame_count: 0,
            video_writer: Some(video_writer),
            save_frame: true,
        }
    }

    /// The size of the camera frame
    ///
    /// for a `scene`, this is equal to `scene.camera.frame.size`
    pub fn size(&self) -> (usize, usize) {
        self.camera.frame.size
    }

    /// With default [`FileWriterBuilder`]
    pub fn new() -> Self {
        Self::new_with_video_writer_builder(FileWriter::builder())
    }

    pub fn render_to_image(&mut self, path: impl AsRef<Path>) {
        self.extract();
        self.prepare();
        self.render();
        self.save_frame_to_image(path);
    }

    pub fn update_frame(&mut self, update: bool) {
        // TODO: solve the problem that the new inserted rabjects needs update
        if update || true {
            self.extract();
            self.prepare();
        }
        self.render();
        if let Some(writer) = &mut self.video_writer {
            writer.write_frame(&self.camera.get_rendered_texture(&self.ctx.wgpu_ctx));
        }
        if self.save_frame {
            let path = format!("output/image-{:04}.png", self.frame_count);
            let dir = Path::new(&path).parent().unwrap();
            if !dir.exists() {
                fs::create_dir_all(dir).unwrap();
            }
            self.save_frame_to_image(path);
        }
        self.frame_count += 1;
    }

    pub fn save_frame_to_image(&mut self, path: impl AsRef<Path>) {
        info!("[Scene]: SAVE FRAME TO IMAGE START");
        let t = Instant::now();
        let size = self.camera.frame.size;
        let texture_data = self.camera.get_rendered_texture(&self.ctx.wgpu_ctx);
        let buffer: ImageBuffer<Rgba<u8>, &[u8]> =
            ImageBuffer::from_raw(size.0 as u32, size.1 as u32, texture_data).unwrap();
        buffer.save(path).unwrap();
        info!("[Scene]: SAVE FRAME TO IMAGE END, took {:?}", t.elapsed());
    }

    pub fn tick_duration(&self) -> Duration {
        Duration::from_secs_f32(1.0 / self.camera.fps as f32)
    }

    pub fn insert_updater<R: Rabject + 'static, U: Updater<R> + 'static>(
        &mut self,
        target_id: RabjectId<R>,
        mut updater: U,
    ) {
        {
            let target = self.get_mut::<R>(target_id).unwrap();
            updater.on_create(target);
        }
        let updater = Box::new(updater);
        let entry = self
            .updaters
            .entry(TypeId::of::<R>())
            .or_insert(Box::new(Vec::<(Id, UpdaterStore<R>)>::new()));
        entry
            .downcast_mut::<Vec<(Id, UpdaterStore<R>)>>()
            .unwrap()
            .push((*target_id, UpdaterStore { updater, target_id }));
    }

    // /// Play an animation
    // ///
    // /// See [`Animation`].
    // pub fn play<R: Rabject>(&mut self, animation: Animation<R>) -> Option<RabjectWithId<R>> {
    //     // trace!(
    //     //     "[Scene] Playing animation on {:?}...",
    //     //     animation.rabject.id()
    //     // );
    //     animation.play(self)
    // }

    /// Advance the scene by a given duration
    pub fn advance(&mut self, duration: Duration) {
        let dt = self.tick_duration().as_secs_f32();
        let frames = (duration.as_secs_f32() / dt).ceil() as usize;

        for _ in 0..frames {
            self.tick(dt);
            self.update_frame(true);
        }
    }

    /// Keep the scene static for a given duration
    pub fn wait(&mut self, duration: Duration) {
        let dt = self.tick_duration().as_secs_f32();
        let frames = (duration.as_secs_f32() / dt).ceil() as usize;

        for _ in 0..frames {
            self.update_frame(false);
        }
    }
}
