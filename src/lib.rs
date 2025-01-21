//! Ranim is an animation engine written in rust, using [`wgpu`] and [`vello`].
//!
//! # Getting Started
//!
//! Every thing starts with a [`scene::Scene`], which handles the update
//! and render of [`scene::Entity`]s.
//!
//! To create a scene, use [`scene::SceneBuilder`]:
//!
//! ```rust
//! let mut scene = SceneBuilder::new("scene-hello").build();
//! ```
//!
//! A scene implements [`std::ops::Deref`] to a [`scene::EntityStore`], which stores entities.
//!
//! You can treat it as a [`HashMap`](std::collections::HashMap) with [`scene::EntityId`] as the key,
//! and the [`scene::Entity`] as the value, it has the following methods:
//! - [`insert`](scene::EntityStore::insert):
//!   Insert an [`Entity`](scene::Entity) into the store, and returns a new [`EntityId`](scene::EntityId)
//! - [`remove`](scene::EntityStore::remove):
//!   Consumes an [`EntityId`](scene::EntityId) and remove the corresponding [`Entity`](scene::Entity) from the store.
//! - [`get`](scene::EntityStore::get) and [`get_mut`](scene::EntityStore::get_mut):
//!   Get the reference of an [`Entity`](scene::Entity) by a reference of [`EntityId`](scene::EntityId).
//!
//! The [`scene::Scene`] in Ranim is 3d, it can only render 3d objects.
//! However, there is a similar structure called [`scene::canvas::Canvas`], which is basically
//! a 2d scene and can render 2d objects while itself can be rendered in 3d scene as a quad with texture.
//!
//! To create a canvas and add it into the scene, use [`scene::Scene::insert_new_canvas`]:
//!
//! ```rust
//! // Create a canvas with viewport size 1920x1080
//! let canvas = scene.insert_new_canvas(1920, 1080);
//! ```
//!
//! The easiest way to create an entity is to use [`rabject::Blueprint`], its basically a builder of [`Entity`]:
//!
//! ```rust
//! {
//!     let canvas = scene.get_mut(canvas);
//!
//!     // Create a VMobject with a closed path formed by 5 points
//!     let mut polygon = Polygon::new(vec![
//!         vec2(0.0, 0.0),
//!         vec2(-100.0, -300.0),
//!         vec2(500.0, 0.0),
//!         vec2(0.0, 700.0),
//!         vec2(200.0, 300.0),
//!     ])
//!     .with_stroke_width(10.0)
//!     .build();
//!     // Set the properties of the rabject
//!     polygon.set_color(Srgba::hex("FF8080FF").unwrap()).rotate(
//!         std::f32::consts::PI / 4.0,
//!         Vec3::Z,
//!         TransformAnchor::origin(),
//!     );
//!
//!     // Insert the VMobject and get its id
//!     let polygon = canvas.insert(polygon);
//! }
//! ```
//!
//! Now the [`scene::Scene`] contains a [`scene::canvas::Canvas`], and in the canvas,
//! there is a [`scene::Entity`] built by the [`crate::rabject::rabject2d::vmobject::geometry::Polygon`] blueprint.
//!
//! Finally, to render the scene, use [`scene::Scene::render_to_image`]:
//!
//! ```rust
//! scene.render_to_image("hello.png");
//! ```
//!
//! Then you will find `hello.png` in `output/scene-hello/hello.png`.

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use animation::{AnimateTarget, Animation};
use context::RanimContext;
use file_writer::{FileWriter, FileWriterBuilder};
pub use glam;
use image::{ImageBuffer, Rgba};
use log::trace;
use render::{CameraFrame, Renderer};
use world::{
    // canvas::{camera::CanvasCamera, Canvas},
    Entity,
    EntityId,
    Store,
    World,
};
pub mod prelude {
    pub use crate::interpolate::Interpolatable;

    pub use crate::animation::creation::{Empty, Fill, Partial, Stroke};
    pub use crate::animation::fading::Opacity;
    pub use crate::animation::transform::Alignable;

    pub use crate::rabject::Blueprint;
}

pub mod color;
mod file_writer;
mod interpolate;
pub mod updater;

pub mod animation;
pub mod components;
pub mod context;
pub mod items;
pub mod rabject;
pub mod render;
pub mod utils;
pub mod world;

pub struct SceneDesc {
    pub name: String,
}

pub trait Scenee {
    fn desc() -> SceneDesc;
    fn construct<T: RanimApp>(&mut self, app: &mut T);
}

pub trait RanimApp: Store<Renderer> {
    /// Play an animation
    ///
    /// The different target_type is corresponding to different [`AnimateTarget`]:
    /// - [`Entity`] represents an entity that is not existed in the scene
    ///
    ///   It corresponds to [`AnimateTarget::Insert`], the entity will
    ///   be inserted into the scene before animation plays
    /// - [`EntityId`] represents an entity that is existed in the scene
    ///   It corresponds to [`AnimateTarget::Existed`]
    ///
    /// Should note that this function takes the ownership of the entity or its id and returns the id.
    ///
    /// If you want to remove the entity after animation plays, you can use [`Scene::play_remove`].
    ///
    /// For playing animation in a canvas, see [`Scene::play_in_canvas`] and [`Scene::play_remove_in_canvas`].
    ///
    /// The actual "playing" part is equal to:
    /// ```rust
    /// let run_time = animation.config.run_time;
    /// self.get_mut(&entity_id).insert_updater(animation);
    /// self.advance(run_time);
    /// ```
    ///
    /// See [`Animation`] and [`crate::updater::Updater`].
    fn play<E: Entity<Renderer = Renderer> + 'static, T: Into<AnimateTarget<E>>>(
        &mut self,
        target: T,
        animation: Animation<E>,
    ) -> EntityId<E>;
    fn play_remove<E: Entity<Renderer = Renderer> + 'static>(
        &mut self,
        target_id: EntityId<E>,
        animation: Animation<E>,
    );

    // /// Play an animation in a canvas
    // ///
    // /// Same like [`Scene::play`], but the animation will be played in a canvas
    // ///
    // /// See [`Animation`] and [`crate::updater::Updater`].
    // fn play_in_canvas<E: Entity<Renderer = CanvasCamera> + 'static, T: Into<AnimateTarget<E>>>(
    //     &mut self,
    //     canvas_id: &EntityId<Canvas>,
    //     target: T,
    //     animation: Animation<E>,
    // ) -> EntityId<E>;
    // fn play_remove_in_canvas<E: Entity<Renderer = CanvasCamera> + 'static>(
    //     &mut self,
    //     canvas_id: &EntityId<Canvas>,
    //     target_id: EntityId<E>,
    //     animation: Animation<E>,
    // );
    // fn center_canvas_in_frame(&mut self, canvas_id: &EntityId<Canvas>);
    fn wait(&mut self, duration: Duration);
    // fn insert_new_canvas(&mut self, width: u32, height: u32) -> EntityId<Canvas>;

    fn render_to_image(&mut self, filename: impl AsRef<str>);
}

pub struct RanimRenderApp {
    ctx: RanimContext,

    world: World,
    renderer: Renderer,

    camera_frame: CameraFrame,

    /// The writer for the output.mp4 video
    video_writer: Option<FileWriter>,
    /// Whether to auto create a [`FileWriter`] to output the video
    video_writer_builder: Option<FileWriterBuilder>,
    /// Whether to save the frames
    save_frames: bool,
    /// fps
    fps: u32,

    frame_count: u32,
}

pub struct AppOptions {
    pub frame_size: (u32, u32),
    pub frame_rate: u32,
    pub save_frames: bool,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            frame_size: (1920, 1080),
            frame_rate: 60,
            save_frames: false,
        }
    }
}

impl RanimRenderApp {
    pub fn new(options: AppOptions) -> Self {
        let ctx = RanimContext::new();
        let camera_frame = CameraFrame::new_with_size(
            options.frame_size.0 as usize,
            options.frame_size.1 as usize,
        );
        let mut renderer = Renderer::new(
            &ctx,
            options.frame_size.0 as usize,
            options.frame_size.1 as usize,
        );
        renderer.update_uniforms(&ctx.wgpu_ctx, &camera_frame);
        Self {
            world: World::new(),
            renderer,
            camera_frame,
            video_writer: None,
            video_writer_builder: Some(
                FileWriterBuilder::default()
                    .with_fps(options.frame_rate)
                    .with_size(options.frame_size.0, options.frame_size.1),
            ),
            save_frames: options.save_frames,
            fps: options.frame_rate,
            frame_count: 0,
            ctx,
        }
    }
    fn tick_duration(&self) -> Duration {
        Duration::from_secs_f32(1.0 / self.fps as f32)
    }

    /// Advance the scene by a given duration
    ///
    /// this method writes frames through [`Self::update_frame`]
    fn advance(&mut self, duration: Duration) {
        let dt = self.tick_duration().as_secs_f32();
        let frames = (duration.as_secs_f32() / dt).ceil() as usize;

        for _ in 0..frames {
            let start = Instant::now();
            self.world.tick(dt);
            trace!("[Scene/advance] tick cost: {:?}", start.elapsed());
            let t = Instant::now();
            self.update_frame(true);
            trace!("[Scene/advance] update_frame cost: {:?}", t.elapsed());
            trace!(
                "[Scene/advance] one complete frame cost: {:?}",
                start.elapsed()
            );
        }
    }

    fn update_frame(&mut self, update: bool) {
        // TODO: solve the problem that the new inserted rabjects needs update
        if update || true {
            self.world.extract();
            self.world.prepare(&self.ctx);
        }
        self.renderer
            .render(&mut self.ctx, &mut self.world.entities);

        // `output_video` is true
        if let Some(video_writer) = self.video_writer.as_mut() {
            video_writer.write_frame(self.renderer.get_rendered_texture_data(&self.ctx.wgpu_ctx));
        } else if let Some(builder) = self.video_writer_builder.as_ref() {
            self.video_writer
                .get_or_insert(builder.clone().build())
                .write_frame(self.renderer.get_rendered_texture_data(&self.ctx.wgpu_ctx));
        }

        // `save_frames` is true
        if self.save_frames {
            let path = format!("output/{}/frames/{:04}.png", "scene", self.frame_count);
            self.save_frame_to_image(path);
        }
        self.frame_count += 1;
    }

    // pub fn render_to_image(&mut self, world: &mut World, filename: impl AsRef<str>) {
    //     let filename = filename.as_ref();
    //     world.extract();
    //     world.prepare(&self.ctx);
    //     self.renderer.render(&mut self.ctx, &mut world.entities);
    //     self.save_frame_to_image(PathBuf::from(format!("output/{}/{}", "world", filename)));
    // }

    pub fn save_frame_to_image(&mut self, path: impl AsRef<Path>) {
        let dir = path.as_ref().parent().unwrap();
        if !dir.exists() {
            std::fs::create_dir_all(dir).unwrap();
        }
        // info!("[Scene]: SAVE FRAME TO IMAGE START");
        // let t = Instant::now();
        let size = self.camera_frame.size;
        let texture_data = self.renderer.get_rendered_texture_data(&self.ctx.wgpu_ctx);
        let buffer: ImageBuffer<Rgba<u8>, &[u8]> =
            ImageBuffer::from_raw(size.0 as u32, size.1 as u32, texture_data).unwrap();
        buffer.save(path).unwrap();
        // info!("[Scene]: SAVE FRAME TO IMAGE END, took {:?}", t.elapsed());
    }
}

impl Store<Renderer> for RanimRenderApp {
    fn insert<E: world::EntityAny<Renderer = Renderer>>(
        &mut self,
        entity: E,
    ) -> world::EntityId<E> {
        self.world.insert(entity)
    }
    fn remove<E: world::EntityAny<Renderer = Renderer>>(&mut self, id: world::EntityId<E>) {
        self.world.remove(id);
    }
    fn get<E: world::EntityAny<Renderer = Renderer>>(
        &self,
        id: &world::EntityId<E>,
    ) -> &world::EntityStore<E> {
        self.world.get(id)
    }
    fn get_mut<E: world::EntityAny<Renderer = Renderer>>(
        &mut self,
        id: &world::EntityId<E>,
    ) -> &mut world::EntityStore<E> {
        self.world.get_mut(id)
    }
}

impl RanimApp for RanimRenderApp {
    fn play<E: Entity<Renderer = Renderer> + 'static, T: Into<AnimateTarget<E>>>(
        &mut self,
        target: T,
        animation: Animation<E>,
    ) -> EntityId<E> {
        let target = target.into();

        let entity_id = match target {
            AnimateTarget::Insert(entity) => self.insert(entity),
            AnimateTarget::Existed(entity_id) => entity_id,
        };

        let run_time = animation.config.run_time;
        self.get_mut(&entity_id).insert_updater(animation);
        self.advance(run_time);
        entity_id
    }

    fn play_remove<E: Entity<Renderer = Renderer> + 'static>(
        &mut self,
        target_id: EntityId<E>,
        animation: Animation<E>,
    ) {
        let target_id = self.play(target_id, animation);
        self.remove(target_id);
    }

    fn render_to_image(&mut self, filename: impl AsRef<str>) {
        let filename = filename.as_ref();
        self.world.extract();
        self.world.prepare(&self.ctx);
        self.renderer.render(&mut self.ctx, &mut self.world.entities);
        self.save_frame_to_image(PathBuf::from(format!("output/{}/{}", "world", filename)));
    }

    // fn play_in_canvas<E: Entity<Renderer = CanvasCamera> + 'static, T: Into<AnimateTarget<E>>>(
    //     &mut self,
    //     canvas_id: &EntityId<Canvas>,
    //     target: T,
    //     animation: Animation<E>,
    // ) -> EntityId<E> {
    //     let target = target.into();

    //     let entity_id = match target {
    //         AnimateTarget::Insert(entity) => self.get_mut(canvas_id).insert(entity),
    //         AnimateTarget::Existed(entity_id) => entity_id,
    //     };

    //     let run_time = animation.config.run_time;
    //     self.get_mut(canvas_id)
    //         .get_mut(&entity_id)
    //         .insert_updater(animation);
    //     self.advance(run_time);
    //     entity_id
    // }

    // fn play_remove_in_canvas<E: Entity<Renderer = CanvasCamera> + 'static>(
    //     &mut self,
    //     canvas_id: &EntityId<Canvas>,
    //     target_id: EntityId<E>,
    //     animation: Animation<E>,
    // ) {
    //     let target_id = self.play_in_canvas(canvas_id, target_id, animation);
    //     self.get_mut(canvas_id).remove(target_id);
    // }

    fn wait(&mut self, duration: Duration) {
        let dt = self.tick_duration().as_secs_f32();
        let frames = (duration.as_secs_f32() / dt).ceil() as usize;

        for _ in 0..frames {
            let start = Instant::now();
            self.update_frame(false);
            trace!(
                "[Scene/wait] one complete frame(update_frame) cost: {:?}",
                start.elapsed()
            );
        }
    }
    // fn center_canvas_in_frame(&mut self, canvas_id: &EntityId<Canvas>) {
    //     let canvas = self.world.get(canvas_id);
    //     self.camera_frame.center_canvas_in_frame(canvas);
    //     self.renderer
    //         .update_uniforms(&self.ctx.wgpu_ctx, &self.camera_frame);
    // }

    // fn insert_new_canvas(&mut self, width: u32, height: u32) -> EntityId<Canvas> {
    //     let canvas = Canvas::new(&self.ctx.wgpu_ctx, width, height);
    //     self.world.insert(canvas)
    // }
}
