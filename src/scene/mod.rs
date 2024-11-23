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
use log::{trace, warn};

use crate::{
    animation::Animation,
    camera::Camera,
    mobject::Mobject,
    renderer::{vmobject::VMobjectRenderer, Renderer},
    utils::Id,
    RanimContext,
};

pub struct Scene {
    pub camera: Camera,
    /// Mobjects in the scene, they are actually [`crate::mobject::ExtractedMobject`]
    ///
    /// Mobject's renderer id -> Vec<(Mobject's id, Mobject)>
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

    pub fn remove_mobject<R: Renderer>(&mut self, mobject: &Mobject<R>) {
        self.mobjects.iter_mut().for_each(|(_, mobject_vec)| {
            mobject_vec.retain(|(mobject_id, _)| mobject_id != mobject.id());
        });
    }

    pub fn try_add_mobject<R: Renderer>(
        &mut self,
        ctx: &mut RanimContext,
        mobject: &Mobject<R>,
    ) -> anyhow::Result<()> {
        if self.is_mobject_exist(mobject) {
            return Err(anyhow::anyhow!("mobject already exists"));
        }
        let mobject = mobject.extract(&ctx.wgpu_ctx);
        self.mobjects
            .entry(mobject.renderer_id)
            .or_default()
            .push((mobject.id, Box::new(mobject)));
        Ok(())
    }

    pub fn is_mobject_exist<R: Renderer>(&self, mobject: &Mobject<R>) -> bool {
        self.mobjects
            .get(&std::any::TypeId::of::<R>())
            .map(|mobject_vec| mobject_vec.iter().any(|(id, _)| id == mobject.id()))
            .unwrap_or(false)
    }

    pub fn render_to_image(&mut self, ctx: &mut RanimContext, path: impl AsRef<Path>) {
        self.camera
            .render::<VMobjectRenderer>(ctx, &mut self.mobjects);
        self.save_frame_to_image(ctx, path);
    }

    pub fn update_frame(&mut self, ctx: &mut RanimContext, dt: f32) {
        self.time += dt;
        // self.update_mobjects(dt);
        self.camera
            .render::<VMobjectRenderer>(ctx, &mut self.mobjects);
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
    pub fn play<R: Renderer>(
        &mut self,
        ctx: &mut RanimContext,
        mut animation: Animation<R>,
    ) -> Option<Mobject<R>> {
        if let Err(err) = self.try_add_mobject(ctx, &animation.mobject) {
            warn!(
                "[Scene] Failed to add mobject {:?}: {}",
                animation.mobject.id(),
                err
            );
        }
        trace!("[Scene] Playing animation {:?}...", animation.mobject.id());
        // TODO: handle the precision problem
        let frames = animation.config.calc_frames(self.camera.fps as f32);

        let dt = animation.config.run_time.as_secs_f32() / (frames - 1) as f32;
        for t in (0..frames).map(|x| x as f32 * dt) {
            // TODO: implement mobject's updaters
            // animation.update_mobjects(dt);
            let alpha = t / animation.config.run_time.as_secs_f32();
            let alpha = (animation.config.rate_func)(alpha);
            animation.interpolate(alpha);
            self.update_frame(ctx, dt);
            self.frame_count += 1;
        }
        if animation.should_remove() {
            self.remove_mobject(&animation.mobject);
            None
        } else {
            Some(animation.mobject)
        }
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
