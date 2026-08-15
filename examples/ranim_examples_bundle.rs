#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(rustdoc::private_intra_doc_links)]

//! Interactive documentation for all ranim examples.
//!
//! Scenes marked with [`wasm_demo_doc`](ranim::prelude::wasm_demo_doc)
//! embed a live preview into their rustdoc page. Run `just doc-examples`
//! to build this crate's docs together with the wasm bundle.
#[path = "aabb/lib.rs"]
pub mod aabb;
#[path = "animating_pi/lib.rs"]
pub mod animating_pi;
#[path = "arc/lib.rs"]
pub mod arc;
#[path = "arc_between_points/lib.rs"]
pub mod arc_between_points;
#[path = "basic/lib.rs"]
pub mod basic;
#[path = "bubble_sort/lib.rs"]
pub mod bubble_sort;
#[path = "cloth_wrap/lib.rs"]
pub mod cloth_wrap;
#[path = "composable_choreography/lib.rs"]
pub mod composable_choreography;
#[path = "ellipse/lib.rs"]
pub mod ellipse;
#[path = "extract_vitem_visualize/lib.rs"]
pub mod extract_vitem_visualize;
#[path = "getting_started0/lib.rs"]
pub mod getting_started0;
#[path = "getting_started1/lib.rs"]
pub mod getting_started1;
#[path = "getting_started2/lib.rs"]
pub mod getting_started2;
#[path = "hanoi/lib.rs"]
pub mod hanoi;
#[path = "hello_ranim/lib.rs"]
pub mod hello_ranim;
#[path = "iterative_spring/lib.rs"]
pub mod iterative_spring;
#[path = "mesh_morph/lib.rs"]
pub mod mesh_morph;
#[path = "midi_visualizer/lib.rs"]
pub mod midi_visualizer;
#[path = "nbody/lib.rs"]
pub mod nbody;
#[path = "output_formats/lib.rs"]
pub mod output_formats;
#[path = "palettes/lib.rs"]
pub mod palettes;
#[path = "perlin_terrain/lib.rs"]
pub mod perlin_terrain;
#[path = "perspective_blend/lib.rs"]
pub mod perspective_blend;
#[path = "ranim_logo/lib.rs"]
pub mod ranim_logo;
#[path = "regular_polygon/lib.rs"]
pub mod regular_polygon;
#[path = "selective_sort/lib.rs"]
pub mod selective_sort;
#[path = "solar_system/lib.rs"]
pub mod solar_system;
#[path = "test/lib.rs"]
pub mod test;
#[path = "tetrahedron_spheres/lib.rs"]
pub mod tetrahedron_spheres;
#[path = "text_item/lib.rs"]
pub mod text_item;
#[path = "typst_timer/lib.rs"]
pub mod typst_timer;

mod midi {
    pub(crate) use crate::midi_visualizer::midi::*;
}
