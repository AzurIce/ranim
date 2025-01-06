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

pub use glam;
pub mod prelude {
    pub use crate::interpolate::Interpolatable;

    pub use crate::animation::fading::Opacity;
    pub use crate::animation::transform::Alignable;
    pub use crate::animation::creation::{Partial, Empty, Fill, Stroke};
    pub use crate::rabject::rabject2d::BoundingBox;

    pub use crate::rabject::Blueprint;
}

pub mod color;
mod interpolate;
pub mod updater;

pub mod animation;
pub mod context;
/// Rabjects are the basic objects in ranim scene
pub mod rabject;
/// To arrange rabjects
pub mod scene;
pub mod utils;
