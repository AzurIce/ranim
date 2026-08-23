#![allow(dead_code)]
#![allow(unused_imports)]

//! Scene plugin bundle for the wasm engine/plugin split demo.
//!
//! Built as a plain example of the root package with the `scene-plugin`
//! feature, which activates the forwarding allocator and engine imports
//! inside `ranim` itself (see `ranim::` `src/scene_plugin.rs`):
//!
//! ```bash
//! RUSTFLAGS='<plugin link flags>' \
//!     cargo build --example scenes_plugin --target wasm32-unknown-unknown \
//!         --release --features scene-plugin
//! ```
//!
//! The produced raw module imports the engine's memory/table/allocator and
//! exports `scene_cnt()` / `get_scene(idx)` which the JS loader feeds back
//! into the engine's `register_scene`.

#[path = "aabb/lib.rs"]
pub mod aabb;
#[path = "arc/lib.rs"]
pub mod arc;
#[path = "arc_between_points/lib.rs"]
pub mod arc_between_points;
#[path = "bubble_sort/lib.rs"]
pub mod bubble_sort;
#[path = "ellipse/lib.rs"]
pub mod ellipse;
#[path = "hanoi/lib.rs"]
pub mod hanoi;
#[path = "regular_polygon/lib.rs"]
pub mod regular_polygon;
