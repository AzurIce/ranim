pub mod file_writer;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fs,
    path::Path,
    time::Duration,
};

use file_writer::{FileWriter, FileWriterBuilder};
use image::{ImageBuffer, Rgba};
use log::trace;

use crate::{
    animation::Animation,
    camera::Camera,
    rabject::{group::VGroup, vmobject::VMobject, Rabject, RabjectId},
    utils::Id,
    RanimContext,
};

/// An entity in the scene
///
/// rabject --extract--> render_data --init--> render_resource
pub struct Entity<R: Rabject> {
    pub rabject: R,
    pub children: Vec<Id>,
    pub render_data: Option<R::RenderData>,
    pub render_resource: Option<R::RenderResource>,
}

pub struct Scene {
    ctx: RanimContext,
    pub camera: Camera,
    /// Entities in the scene
    ///
    /// Rabject's type id -> Vec<(Rabject's id, Entity<Rabject>)>
    pub entities: HashMap<TypeId, Vec<(Id, Box<dyn Any>)>>,
    /// Rabjects in the scene, they are actually [`crate::rabject::ExtractedRabjectWithId`]
    ///
    /// Rabject's type id -> Vec<(Rabject's id, ExtractedRabject<Rabject>)>
    pub rabjects: HashMap<TypeId, Vec<(Id, Box<dyn Any>)>>,
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
    pub fn insert_entity<R: Rabject + 'static>(&mut self, entity: Entity<R>) -> Id {
        let id = Id::new();
        let entry = self
            .entities
            .entry(std::any::TypeId::of::<R>())
            .or_default();
        entry.push((id, Box::new(entity)));
        id
    }

    /// Low level api to remove an entity from the scene directly
    ///
    /// For high level api, see [`Scene::remove`]
    pub fn remove_entity(&mut self, id: Id) {
        for entry in self.entities.values_mut() {
            entry.retain(|(eid, _)| id != *eid);
        }
    }

    /// Low level api to get reference of an entity from the scene directly
    ///
    /// For high level api, see [`Scene::get`]
    pub fn get_entity<R: Rabject + 'static>(&self, id: Id) -> Option<&Entity<R>> {
        self.entities
            .get(&std::any::TypeId::of::<R>())
            .and_then(|e| {
                e.iter()
                    .find(|(eid, _)| id == *eid)
                    .map(|(_, e)| e.downcast_ref::<Entity<R>>().unwrap())
            })
    }

    /// Low level api to get mutable reference of an entity from the scene directly
    ///
    /// For high level api, see [`Scene::get_mut`]
    pub fn get_entity_mut<R: Rabject + 'static>(&mut self, id: Id) -> Option<&mut Entity<R>> {
        self.entities
            .get_mut(&std::any::TypeId::of::<R>())
            .and_then(|e| {
                e.iter_mut()
                    .find(|(eid, _)| id == *eid)
                    .map(|(_, e)| e.downcast_mut::<Entity<R>>().unwrap())
            })
    }
}

// Entity management - High level apis
impl Scene {
    /// Insert a rabject to the scene
    ///
    /// See [`Rabject::insert_to_scene`]
    pub fn insert<R: Rabject + 'static>(&mut self, rabject: R) -> <R as Rabject>::Id {
        Box::new(rabject).insert_to_scene(self)
    }

    /// Remove a rabject from the scene
    ///
    /// See [`Rabject::remove_from_scene`]
    pub fn remove<R: Rabject + 'static>(&mut self, id: <R as Rabject>::Id) {
        R::remove_from_scene(self, id);
    }

    /// Get a reference of a rabject from the scene
    pub fn get<R: Rabject + 'static>(&self, id: <R as Rabject>::Id) -> Option<&R> {
        self.get_entity::<R>(id.to_id()).map(|e| &e.rabject)
    }

    /// Get a mutable reference of a rabject from the scene
    pub fn get_mut<R: Rabject + 'static>(&mut self, id: <R as Rabject>::Id) -> Option<&mut R> {
        self.get_entity_mut::<R>(id.to_id()).map(|e| &mut e.rabject)
    }
}

// the core phases
impl Scene {
    pub fn extract(&mut self) {
        for (type_id, entities) in self.entities.iter_mut() {
            for (id, entity) in entities.iter_mut() {
                if let Some(entity) = entity.downcast_mut::<Entity<VMobject>>() {
                    entity.render_data = Some(entity.rabject.extract());
                } else if let Some(entity) = entity.downcast_mut::<Entity<VGroup>>() {
                    entity.render_data = Some(entity.rabject.extract());
                }
            }
        }
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
            entities: HashMap::new(),
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

    // pub fn remove_rabject<R: Rabject>(&mut self, rabject: &RabjectWithId<R>) {
    //     trace!(
    //         "[Scene::remove_rabject]: removing rabject: {:?}",
    //         rabject.id()
    //     );
    //     self.rabjects.iter_mut().for_each(|(_, rabject_vec)| {
    //         rabject_vec.retain(|(rabject_id, _)| rabject_id != rabject.id());
    //     });
    // }

    // pub fn insert_group(&mut self, group: &Group) {
    //     for child in group.children.iter() {
    //         if let Some(rabject) = child.downcast_ref::<RabjectWithId<VMobject>>() {
    //             self.insert_rabject(rabject);
    //         }
    //     }
    // }

    // pub fn remove_group(&mut self, group: &Group) {
    //     for child in group.children.iter() {
    //         if let Some(rabject) = child.downcast_ref::<RabjectWithId<VMobject>>() {
    //             self.remove_rabject(rabject);
    //         }
    //     }
    // }

    // pub fn insert_rabject<R: Rabject>(&mut self, rabject: &R) -> Id {
    //     trace!(
    //         "[Scene::insert_rabject]: inserting rabject: {:?}",
    //         rabject.id()
    //     );
    //     let entry = self
    //         .rabjects
    //         .entry(std::any::TypeId::of::<R>())
    //         .or_default();
    //     if let Some((_, extracted)) = entry.iter_mut().find(|(id, _)| id == rabject.id()) {
    //         trace!(
    //             "[Scene::insert_rabject]: already_exist, updating rabject: {:?}",
    //             rabject.id()
    //         );
    //         let extracted: &mut ExtractedRabjectWithId<R> = extracted.downcast_mut().unwrap();
    //         extracted.update(&mut self.ctx, rabject);
    //     } else {
    //         entry.push((*rabject.id(), Box::new(rabject.extract(&mut self.ctx))));
    //     }
    // }

    // pub fn is_rabject_exist<R: Rabject>(&self, rabject: &RabjectWithId<R>) -> bool {
    //     self.rabjects
    //         .get(&std::any::TypeId::of::<R>())
    //         .map(|rabject_vec| rabject_vec.iter().any(|(id, _)| id == rabject.id()))
    //         .unwrap_or(false)
    // }

    pub fn render_to_image(&mut self, path: impl AsRef<Path>) {
        self.camera
            .render::<VMobject>(&mut self.ctx, &mut self.rabjects);
        self.save_frame_to_image(path);
    }

    pub fn update_frame(&mut self, dt: f32) {
        self.time += dt;
        // self.update_mobjects(dt);
        self.camera
            .render::<VMobject>(&mut self.ctx, &mut self.rabjects);
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
        let size = self.camera.frame.size;
        let texture_data = self.camera.get_rendered_texture(&self.ctx.wgpu_ctx);
        let buffer: ImageBuffer<Rgba<u8>, &[u8]> =
            ImageBuffer::from_raw(size.0 as u32, size.1 as u32, texture_data).unwrap();
        buffer.save(path).unwrap();
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
