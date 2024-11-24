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

use crate::{
    animation::Animation,
    camera::Camera,
    rabject::{vmobject::VMobject, ExtractedRabjectWithId, Rabject, RabjectWithId},
    utils::Id,
    RanimContext,
};

pub struct Scene {
    pub camera: Camera,
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

impl Scene {
    pub fn new_with_video_writer_builder(ctx: &RanimContext, builder: FileWriterBuilder) -> Self {
        let camera = Camera::new(
            ctx,
            builder.width as usize,
            builder.height as usize,
            builder.fps,
        );
        let video_writer = builder.build();
        Self {
            camera,
            rabjects: HashMap::new(),
            time: 0.0,
            frame_count: 0,
            video_writer: Some(video_writer),
            save_frame: true,
        }
    }

    /// With default [`FileWriterBuilder`]
    pub fn new(ctx: &RanimContext) -> Self {
        Self::new_with_video_writer_builder(ctx, FileWriter::builder())
    }

    pub fn remove_rabject<R: Rabject>(&mut self, rabject: &RabjectWithId<R>) {
        self.rabjects.iter_mut().for_each(|(_, rabject_vec)| {
            rabject_vec.retain(|(rabject_id, _)| rabject_id != rabject.id());
        });
    }

    pub fn insert_rabject<R: Rabject>(
        &mut self,
        ctx: &mut RanimContext,
        rabject: &RabjectWithId<R>,
    ) {
        let entry = self
            .rabjects
            .entry(std::any::TypeId::of::<R>())
            .or_default();
        if let Some((_, extracted)) = entry.iter_mut().find(|(id, _)| id == rabject.id()) {
            let extracted: &mut ExtractedRabjectWithId<R> = extracted.downcast_mut().unwrap();
            extracted.update_render_resource(ctx, rabject);
        } else {
            entry.push((*rabject.id(), Box::new(rabject.extract(ctx))));
        }
    }

    pub fn is_rabject_exist<R: Rabject>(&self, rabject: &RabjectWithId<R>) -> bool {
        self.rabjects
            .get(&std::any::TypeId::of::<R>())
            .map(|rabject_vec| rabject_vec.iter().any(|(id, _)| id == rabject.id()))
            .unwrap_or(false)
    }

    pub fn render_to_image(&mut self, ctx: &mut RanimContext, path: impl AsRef<Path>) {
        self.camera.render::<VMobject>(ctx, &mut self.rabjects);
        self.save_frame_to_image(ctx, path);
    }

    pub fn update_frame(&mut self, ctx: &mut RanimContext, dt: f32) {
        self.time += dt;
        // self.update_mobjects(dt);
        self.camera.render::<VMobject>(ctx, &mut self.rabjects);
        if let Some(writer) = &mut self.video_writer {
            writer.write_frame(&self.camera.get_rendered_texture(&ctx.wgpu_ctx));
        }
        if self.save_frame {
            let path = format!("output/image-{:04}.png", self.frame_count);
            let dir = Path::new(&path).parent().unwrap();
            if !dir.exists() {
                fs::create_dir_all(dir).unwrap();
            }
            self.save_frame_to_image(ctx, path);
        }
    }

    pub fn save_frame_to_image(&mut self, ctx: &mut RanimContext, path: impl AsRef<Path>) {
        let size = self.camera.frame.size;
        let texture_data = self.camera.get_rendered_texture(&ctx.wgpu_ctx);
        let buffer =
            ImageBuffer::<Rgba<u8>, _>::from_raw(size.0 as u32, size.1 as u32, texture_data)
                .unwrap();
        buffer.save(path).unwrap();
    }

    /// Play an animation
    ///
    /// See [`Animation`].
    pub fn play<R: Rabject>(
        &mut self,
        ctx: &mut RanimContext,
        animation: Animation<R>,
    ) -> Option<RabjectWithId<R>> {
        // trace!(
        //     "[Scene] Playing animation on {:?}...",
        //     animation.rabject.id()
        // );
        animation.play(ctx, self)
    }

    /// Keep the scene static for a given duration
    pub fn wait(&mut self, ctx: &mut RanimContext, duration: Duration) {
        let frames = (duration.as_secs_f32() * self.camera.fps as f32) as usize;

        let dt = duration.as_secs_f32() / (frames - 1) as f32;
        for _ in 0..frames {
            self.update_frame(ctx, dt);
            self.frame_count += 1;
        }
    }
}
