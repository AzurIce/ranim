pub mod file_writer;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fs,
    path::Path,
    time::{Duration, Instant},
};

use file_writer::{FileWriter, FileWriterBuilder};
use image::{ImageBuffer, Rgba};
use log::{debug, info};

use crate::{
    camera::Camera,
    rabject::{
        vgroup::{VGroup, VGroupPrimitive},
        vmobject::{primitive::VMobjectPrimitive, VMobject},
        Primitive, Rabject, RabjectId,
    },
    utils::Id,
    RanimContext,
};

#[allow(unused)]
use log::trace;

/// An entity in the scene
///
/// rabject --extract--> render_data --init--> render_resource
pub struct RabjectStore<R: Rabject> {
    /// The rabject
    pub rabject: R,
    /// The extracted data from the rabject
    pub render_data: Option<R::RenderData>,
    /// The prepared render resource of the rabject
    pub render_resource: Option<R::RenderResource>,
}

pub struct Scene {
    ctx: RanimContext,
    pub camera: Camera,
    /// Entities in the scene
    ///
    /// Rabject's type id -> Vec<(Rabject's id, RabjectStore<Rabject>)>
    pub rabjects: HashMap<TypeId, Vec<(Id, Box<dyn Any>)>>,
    /// Rabjects in the scene, they are actually [`crate::rabject::ExtractedRabjectWithId`]
    ///
    /// Rabject's type id -> Vec<(Rabject's id, ExtractedRabject<Rabject>)>
    // pub rabjects: HashMap<TypeId, Vec<(Id, Box<dyn Any>)>>,
    pub time: f32,
    pub frame_count: usize,

    /// The writer for the output.mp4 video
    pub video_writer: Option<FileWriter>,
    /// Whether to save the frame to 'output/image-x.png'
    pub save_frame: bool,
}

// Entity management - Low level apis
impl Scene {
    /// Low level api to insert an entity to the scene directly
    ///
    /// For high level api, see [`Scene::insert`]
    pub fn insert_entity<R: Rabject + 'static>(&mut self, entity: RabjectStore<R>) -> Id {
        let id = Id::new();
        debug!(
            "[Scene::insert_entity]: inserting entity {:?} of type {:?}",
            id,
            std::any::TypeId::of::<R>()
        );
        let entry = self
            .rabjects
            .entry(std::any::TypeId::of::<R>())
            .or_default();
        entry.push((id, Box::new(entity)));
        id
    }

    /// Low level api to remove an entity from the scene directly
    ///
    /// For high level api, see [`Scene::remove`]
    pub fn remove_entity(&mut self, id: &Id) {
        for entry in self.rabjects.values_mut() {
            entry.retain(|(eid, _)| id != eid);
        }
    }

    /// Low level api to get reference of an entity from the scene directly
    ///
    /// For high level api, see [`Scene::get`]
    pub fn get_entity<R: Rabject + 'static>(&self, id: &Id) -> Option<&RabjectStore<R>> {
        self.rabjects
            .get(&std::any::TypeId::of::<R>())
            .and_then(|e| {
                e.iter()
                    .find(|(eid, _)| id == eid)
                    .map(|(_, e)| e.downcast_ref::<RabjectStore<R>>().unwrap())
            })
    }

    /// Low level api to get mutable reference of an entity from the scene directly
    ///
    /// For high level api, see [`Scene::get_mut`]
    pub fn get_entity_mut<R: Rabject + 'static>(
        &mut self,
        id: &Id,
    ) -> Option<&mut RabjectStore<R>> {
        self.rabjects
            .get_mut(&std::any::TypeId::of::<R>())
            .and_then(|e| {
                e.iter_mut()
                    .find(|(eid, _)| id == eid)
                    .map(|(_, e)| e.downcast_mut::<RabjectStore<R>>().unwrap())
            })
    }
}

// Entity management - High level apis
impl Scene {
    /// Insert a rabject to the scene
    ///
    /// See [`Rabject::insert_to_scene`]
    pub fn insert<R: Rabject + 'static>(&mut self, rabject: R) -> RabjectId<R> {
        let entity = RabjectStore {
            rabject,
            render_data: None,
            render_resource: None,
        };
        RabjectId::from_id(self.insert_entity(entity))
    }

    /// Remove a rabject from the scene
    ///
    /// See [`Rabject::remove_from_scene`]
    pub fn remove<R: Rabject>(&mut self, id: &RabjectId<R>) {
        self.remove_entity(id);
    }

    /// Get a reference of a rabject from the scene
    pub fn get<R: Rabject + 'static>(&self, id: &RabjectId<R>) -> Option<&R> {
        self.get_entity::<R>(id).map(|e| &e.rabject)
    }

    /// Get a mutable reference of a rabject from the scene
    pub fn get_mut<R: Rabject + 'static>(&mut self, id: &RabjectId<R>) -> Option<&mut R> {
        self.get_entity_mut::<R>(id).map(|e| &mut e.rabject)
    }
}

// the core phases
impl Scene {
    pub fn extract(&mut self) {
        info!("[Scene]: EXTRACT STAGE START");
        let t = Instant::now();
        for (_, entities) in self.rabjects.iter_mut() {
            for (_, entity) in entities.iter_mut() {
                if let Some(entity) = entity.downcast_mut::<RabjectStore<VMobject>>() {
                    entity.render_data = Some(entity.rabject.extract());
                } else if let Some(entity) = entity.downcast_mut::<RabjectStore<VGroup>>() {
                    entity.render_data = Some(entity.rabject.extract());
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
            rabjects: HashMap::new(),
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

    pub fn update_frame(&mut self, dt: f32) {
        self.time += dt;
        // self.update_mobjects(dt);
        self.extract();
        self.prepare();
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

    /// Keep the scene static for a given duration
    pub fn wait(&mut self, duration: Duration) {
        let frames = (duration.as_secs_f32() * self.camera.fps as f32) as usize;

        let dt = duration.as_secs_f32() / (frames - 1) as f32;
        for _ in 0..frames {
            self.update_frame(dt);
            self.frame_count += 1;
        }
    }
}
