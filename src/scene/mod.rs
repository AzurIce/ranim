pub mod file_writer;
pub mod store;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use file_writer::{FileWriter, FileWriterBuilder};
use image::{ImageBuffer, Rgba};
use store::{Entity, RabjectStore, RabjectStores};

#[allow(unused_imports)]
use log::{debug, info};
#[allow(unused_imports)]
use std::time::Instant;

use crate::{
    animation::Animation,
    camera::Camera,
    context::RanimContext,
    rabject::{
        group::{Group, GroupPrimitive},
        svg_mobject::{SvgMobject, SvgPrimitive},
        vgroup::{VGroup, VGroupPrimitive},
        vmobject::{primitive::VMobjectPrimitive, VMobject},
        vpath::{primitive::VPathPrimitive, VPath},
        Primitive, Rabject, RabjectContainer, RabjectId,
    },
    updater::Updater,
    utils::Id,
};

#[allow(unused)]
use log::trace;
pub struct UpdaterStore<R: Rabject> {
    /// The updater
    pub updater: Box<dyn Updater<R>>,
    /// The id of the target rabject
    pub target_id: RabjectId<R>,
}

/// A builder for [`Scene`]
pub struct SceneBuilder {
    /// The name of the scene (default: "scene")
    ///
    /// This will be used to name the output files
    pub name: String,
    /// The size of the scene (default: (1920, 1080))
    pub size: (usize, usize),
    /// The fps of the scene (default: 60)
    pub fps: u32,
    /// Interactive mode (WIP) (default: false)
    pub interactive: bool,
    /// Whether to output a video (default: true)
    ///
    /// If this is `true`, then the output video will be saved to `output/<name>/output.mp4`
    pub output_video: bool,
    /// Whether to save frames (default: false)
    ///
    /// If this is `true`, then the output frames will be saved to `output/<name>/frames/<frame_count>.png`
    pub save_frames: bool,
}

impl Default for SceneBuilder {
    fn default() -> Self {
        Self {
            name: "scene".to_string(),
            size: (1920, 1080),
            fps: 60,
            interactive: false,
            output_video: true,
            save_frames: false,
        }
    }
}

impl SceneBuilder {
    /// Create a new [`SceneBuilder`] with the scene name
    ///
    /// The name will be used to name the output files
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }
    pub fn with_size(mut self, size: (usize, usize)) -> Self {
        self.size = size;
        self
    }
    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }
    pub fn enable_interactive(mut self) -> Self {
        self.interactive = true;
        self
    }
    pub fn with_output_video(mut self, output_video: bool) -> Self {
        self.output_video = output_video;
        self
    }
    pub fn with_save_frames(mut self, save_frames: bool) -> Self {
        self.save_frames = save_frames;
        self
    }
    pub fn build(self) -> Scene {
        let mut scene = Scene::new(self.name.clone(), self.size.0, self.size.1, self.fps);
        if self.output_video {
            scene.video_writer_builder = Some(
                FileWriter::builder()
                    .with_file_path(PathBuf::from(format!("output/{}/output.mp4", self.name)))
                    .with_size(self.size.0 as u32, self.size.1 as u32)
                    .with_fps(self.fps),
            );
        }
        scene.save_frames = self.save_frames;
        scene
    }
}

pub struct Scene {
    ctx: RanimContext,
    /// The name of the scene
    pub name: String,
    pub camera: Camera,
    /// Rabjects in the scene
    pub rabjects: RabjectStores,
    // /// Updaters for the rabjects
    // ///
    // /// Rabject's type id -> Vec<(Updater's id, Updater<Rabject>)>
    // pub updaters: HashMap<TypeId, Box<dyn Any>>,

    pub time: f32,
    pub frame_count: usize,

    /// The writer for the output.mp4 video
    pub video_writer: Option<FileWriter>,
    /// Whether to auto create a [`FileWriter`] to output the video
    video_writer_builder: Option<FileWriterBuilder>,
    /// Whether to save the frames
    pub save_frames: bool,
}

impl RabjectContainer for Scene {
    /// Insert a rabject to the scene
    ///
    /// See [`RabjectStores::insert`]
    fn update_or_insert<R: Rabject + 'static>(&mut self, rabject: R) -> RabjectId<R> {
        self.rabjects.update_or_insert(rabject)
    }

    /// Remove a rabject from the scene
    ///
    /// See [`RabjectStores::remove`]
    fn remove<R: Rabject>(&mut self, id: RabjectId<R>) {
        self.rabjects.remove(id);
    }

    /// Get a reference of a rabject from the scene
    ///
    /// See [`RabjectStores::get`]
    fn get<R: Rabject + 'static>(&self, id: &RabjectId<R>) -> Option<&RabjectStore<R>> {
        self.rabjects.get(id)
    }

    /// Get a mutable reference of a rabject from the scene
    ///
    /// See [`RabjectStores::get_mut`]
    fn get_mut<R: Rabject + 'static>(&mut self, id: &RabjectId<R>) -> Option<&mut RabjectStore<R>> {
        self.rabjects.get_mut(id)
    }
}

// Core phases
impl Scene {
    pub fn tick(&mut self, dt: f32) {
        // info!("[Scene]: TICK STAGE START");
        // let t = Instant::now();
        self.time += dt;
        for (_, entities) in self.rabjects.iter_mut() {
            for (_, entity) in entities.iter_mut() {
                entity.tick(dt);
            }
        }
        // info!("[Scene]: TICK STAGE END, took {:?}", t.elapsed());
    }

    pub fn extract(&mut self) {
        // info!("[Scene]: EXTRACT STAGE START");
        // let t = Instant::now();
        for (_, entities) in self.rabjects.iter_mut() {
            for (_, entity) in entities.iter_mut() {
                entity.extract();
            }
        }
        // info!("[Scene]: EXTRACT STAGE END, took {:?}", t.elapsed());
    }

    pub fn prepare(&mut self) {
        // info!("[Scene]: PREPARE STAGE START");
        // let t = Instant::now();
        for (_, entities) in self.rabjects.iter_mut() {
            for (_, entity) in entities.iter_mut() {
                entity.prepare(&self.ctx.wgpu_ctx);
            }
        }
        // info!("[Scene]: PREPARE STAGE END, took {:?}", t.elapsed());
    }

    pub fn render(&mut self) {
        // info!("[Scene]: RENDER STAGE START");
        // let t = Instant::now();
        self.camera.update_uniforms(&self.ctx.wgpu_ctx);
        self.camera.clear_screen(&self.ctx.wgpu_ctx);
        self.camera.render(&mut self.ctx, &mut self.rabjects);
        // info!("[Scene]: RENDER STAGE END, took {:?}", t.elapsed());
    }
}

impl Default for Scene {
    fn default() -> Self {
        let ctx = RanimContext::new();
        Self {
            name: "scene".to_string(),

            camera: Camera::new(&ctx, 1920, 1080, 60),
            rabjects: RabjectStores::default(),
            // updaters: HashMap::new(),
            time: 0.0,
            frame_count: 0,
            video_writer: None,
            video_writer_builder: Some(FileWriterBuilder::default()),
            save_frames: false,

            ctx,
        }
    }
}

impl Scene {
    pub fn builder() -> SceneBuilder {
        SceneBuilder::default()
    }

    /// With default [`FileWriterBuilder`]
    pub(crate) fn new(name: impl Into<String>, width: usize, height: usize, fps: u32) -> Self {
        let name = name.into();

        let mut scene = Self::default();
        scene.name = name;
        scene.camera = Camera::new(&scene.ctx, width, height, fps);
        scene
    }

    /// The size of the camera frame
    ///
    /// for a `scene`, this is equal to `scene.camera.frame.size`
    pub fn size(&self) -> (usize, usize) {
        self.camera.frame.size
    }

    pub fn render_to_image(&mut self, filename: impl AsRef<str>) {
        let filename = filename.as_ref();
        self.extract();
        self.prepare();
        self.render();
        self.save_frame_to_image(PathBuf::from(format!("output/{}/{}", self.name, filename)));
    }

    pub fn update_frame(&mut self, update: bool) {
        // TODO: solve the problem that the new inserted rabjects needs update
        if update || true {
            self.extract();
            self.prepare();
        }
        self.render();

        // `output_video` is true
        if let Some(video_writer) = self.video_writer.as_mut() {
            video_writer.write_frame(self.camera.get_rendered_texture(&self.ctx.wgpu_ctx));
        } else if let Some(builder) = self.video_writer_builder.as_ref() {
            self.video_writer.get_or_insert(builder.clone().build());
        }

        // `save_frames` is true
        if self.save_frames {
            let path = format!("output/{}/frames/{:04}.png", self.name, self.frame_count);
            self.save_frame_to_image(path);
        }
        self.frame_count += 1;
    }

    pub fn save_frame_to_image(&mut self, path: impl AsRef<Path>) {
        let dir = path.as_ref().parent().unwrap();
        if !dir.exists() {
            fs::create_dir_all(dir).unwrap();
        }
        // info!("[Scene]: SAVE FRAME TO IMAGE START");
        // let t = Instant::now();
        let size = self.camera.frame.size;
        let texture_data = self.camera.get_rendered_texture(&self.ctx.wgpu_ctx);
        let buffer: ImageBuffer<Rgba<u8>, &[u8]> =
            ImageBuffer::from_raw(size.0 as u32, size.1 as u32, texture_data).unwrap();
        buffer.save(path).unwrap();
        // info!("[Scene]: SAVE FRAME TO IMAGE END, took {:?}", t.elapsed());
    }

    pub fn tick_duration(&self) -> Duration {
        Duration::from_secs_f32(1.0 / self.camera.fps as f32)
    }

    // /// Insert an updater for a target rabject
    // pub fn insert_updater<R: Rabject + 'static, U: Updater<R> + 'static>(
    //     &mut self,
    //     target_id: &RabjectId<R>,
    //     mut updater: U,
    // ) {
    //     {
    //         let target = self.get_mut::<R>(target_id).unwrap();
    //         updater.on_create(target);
    //     }
    //     let updater = Box::new(updater);
    //     let entry = self
    //         .updaters
    //         .entry(TypeId::of::<R>())
    //         .or_insert(Box::new(Vec::<(Id, UpdaterStore<R>)>::new()));
    //     entry
    //         .downcast_mut::<Vec<(Id, UpdaterStore<R>)>>()
    //         .unwrap()
    //         .push((
    //             **target_id,
    //             UpdaterStore {
    //                 updater,
    //                 target_id: *target_id,
    //             },
    //         ));
    // }

    // /// Remove an updater for a target rabject
    // pub fn remove_updater<R: Rabject + 'static>(&mut self, target_id: RabjectId<R>) {
    //     let entry = self
    //         .updaters
    //         .entry(TypeId::of::<R>())
    //         .or_insert(Box::new(Vec::<(Id, UpdaterStore<R>)>::new()));
    //     entry
    //         .downcast_mut::<Vec<(Id, UpdaterStore<R>)>>()
    //         .unwrap()
    //         .retain(|(id, _)| *id != *target_id);
    // }

    /// Play an animation
    ///
    /// This is equal to:
    /// ```rust
    /// let run_time = animation.config.run_time.clone();
    /// scene.insert_updater(target_id, animation);
    /// scene.advance(run_time);
    /// ```
    ///
    /// See [`Animation`] and [`Updater`].
    pub fn play<R: Rabject + 'static>(
        &mut self,
        target_id: &RabjectId<R>,
        animation: Animation<R>,
    ) {
        let run_time = animation.config.run_time;
        self.get_mut(target_id).unwrap().insert_updater(animation);
        // self.insert_updater(target_id, animation);
        self.advance(run_time);
    }

    pub fn play_remove<R: Rabject + 'static>(
        &mut self,
        target_id: RabjectId<R>,
        animation: Animation<R>,
    ) {
        self.play(&target_id, animation);
        self.remove(target_id);
    }

    /// Advance the scene by a given duration
    ///
    /// this method writes frames
    pub fn advance(&mut self, duration: Duration) {
        let dt = self.tick_duration().as_secs_f32();
        let frames = (duration.as_secs_f32() / dt).ceil() as usize;

        for _ in 0..frames {
            self.tick(dt);
            self.update_frame(true);
        }
    }

    /// Keep the scene static for a given duration
    ///
    /// this method writes frames
    pub fn wait(&mut self, duration: Duration) {
        let dt = self.tick_duration().as_secs_f32();
        let frames = (duration.as_secs_f32() / dt).ceil() as usize;

        for _ in 0..frames {
            self.update_frame(false);
        }
    }
}
