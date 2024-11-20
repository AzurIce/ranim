pub mod file_writer;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fs,
    path::Path,
};

use file_writer::{FileWriter, FileWriterBuilder};
use image::{ImageBuffer, Rgba};
use log::trace;

use crate::{
    animation::Animation,
    camera::Camera,
    mobject::Mobject,
    pipeline::{simple, PipelineVertex},
    utils::Id,
    RanimContext,
};

pub struct Scene {
    pub camera: Camera,
    /// Mobjects in the scene, they are actually [`crate::mobject::ExtractedMobject`]
    ///
    /// (Mobject's id, Mobject's pipeline id, Mobject)
    pub mobjects: HashMap<TypeId, Vec<(Id, Box<dyn Any>)>>,
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
            mobjects: HashMap::new(),
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

    pub fn remove_mobject(&mut self, id: Id) {
        self.mobjects.iter_mut().for_each(|(_, mobject_vec)| {
            mobject_vec.retain(|(mobject_id, _)| mobject_id != &id);
        });
    }

    pub fn add_mobject<Vertex: PipelineVertex>(
        &mut self,
        ctx: &mut RanimContext,
        mobject: &Mobject<Vertex>,
    ) {
        let mobject = mobject.extract(&ctx.wgpu_ctx);

        let mobject_vec = self.mobjects.entry(mobject.pipeline_id).or_default();
        mobject_vec.retain(|(id, _)| id != &mobject.id);
        mobject_vec.push((mobject.id, Box::new(mobject)));
    }

    pub fn add_mobjects<Vertex: PipelineVertex>(
        &mut self,
        ctx: &mut RanimContext,
        mobjects: Vec<Mobject<Vertex>>,
    ) {
        // Should be faster?
        self.mobjects.iter_mut().for_each(|(_, mobject_vec)| {
            mobject_vec.retain(|(id, _)| !mobjects.iter().any(|m| id == &m.id))
        });
        mobjects.iter().for_each(|m| {
            let mobject = m.extract(&ctx.wgpu_ctx);
            self.mobjects
                .entry(mobject.pipeline_id)
                .or_default()
                .push((mobject.id, Box::new(mobject)));
        });
    }

    pub fn render_to_image(&mut self, ctx: &mut RanimContext, path: impl AsRef<Path>) {
        self.camera
            .render::<simple::Pipeline>(ctx, &mut self.mobjects);
        self.save_frame_to_image(ctx, path);
    }

    pub fn update_frame(&mut self, ctx: &mut RanimContext, dt: f32) {
        self.time += dt;
        // self.update_mobjects(dt);
        self.camera
            .render::<simple::Pipeline>(ctx, &mut self.mobjects);
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
    pub fn play(
        &mut self,
        ctx: &mut RanimContext,
        mut animation: Animation,
    ) -> Option<Mobject<simple::Vertex>> {
        trace!("[Scene] Playing animation {:?}...", animation.mobject.id);
        // TODO: handle the precision problem
        let frames = animation.config.calc_frames(self.camera.fps as f32);

        let dt = animation.config.run_time.as_secs_f32() / (frames - 1) as f32;
        for t in (0..frames).map(|x| x as f32 * dt) {
            // TODO: implement mobject's updaters
            // animation.update_mobjects(dt);
            let alpha = t / animation.config.run_time.as_secs_f32();
            let alpha = (animation.config.rate_func)(alpha);
            animation.interpolate(alpha);
            self.add_mobject(ctx, &animation.mobject);
            self.update_frame(ctx, dt);
            self.frame_count += 1;
        }
        if animation.should_remove() {
            self.remove_mobject(animation.mobject.id);
            None
        } else {
            Some(animation.mobject)
        }
    }
}
