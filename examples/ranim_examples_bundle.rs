#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(rustdoc::private_intra_doc_links)]

//! Interactive documentation for all ranim examples.
//!
//! Scenes marked with [`wasm_demo_doc`](ranim::prelude::wasm_demo_doc)
//! embed a live preview into their rustdoc page. Run `just doc-examples`
//! to build this crate's docs together with the `ranim-examples-preview`
//! WASM host. This library defines the example plugins; the binary creates
//! the application and starts previews.
#[path = "aabb/scenes.rs"]
pub mod aabb;
#[path = "animating_pi/scenes.rs"]
pub mod animating_pi;
#[path = "arc/scenes.rs"]
pub mod arc;
#[path = "arc_between_points/scenes.rs"]
pub mod arc_between_points;
#[path = "basic/scenes.rs"]
pub mod basic;
#[path = "bubble_sort/scenes.rs"]
pub mod bubble_sort;
#[path = "cloth_wrap/scenes.rs"]
pub mod cloth_wrap;
#[path = "composable_choreography/scenes.rs"]
pub mod composable_choreography;
#[path = "ellipse/scenes.rs"]
pub mod ellipse;
#[path = "extract_vitem_visualize/scenes.rs"]
pub mod extract_vitem_visualize;
#[path = "getting_started0/scenes.rs"]
pub mod getting_started0;
#[path = "getting_started1/scenes.rs"]
pub mod getting_started1;
#[path = "getting_started2/scenes.rs"]
pub mod getting_started2;
#[path = "hanoi/scenes.rs"]
pub mod hanoi;
#[path = "hello_ranim/scenes.rs"]
pub mod hello_ranim;
#[path = "iterative_spring/scenes.rs"]
pub mod iterative_spring;
#[path = "mesh_morph/scenes.rs"]
pub mod mesh_morph;
#[path = "midi_visualizer/scenes.rs"]
pub mod midi_visualizer;
#[path = "nbody/scenes.rs"]
pub mod nbody;
#[path = "output_formats/scenes.rs"]
pub mod output_formats;
#[path = "palettes/scenes.rs"]
pub mod palettes;
#[path = "perlin_terrain/scenes.rs"]
pub mod perlin_terrain;
#[path = "perspective_blend/scenes.rs"]
pub mod perspective_blend;
#[path = "ranim_logo/scenes.rs"]
pub mod ranim_logo;
#[path = "regular_polygon/scenes.rs"]
pub mod regular_polygon;
#[path = "selective_sort/scenes.rs"]
pub mod selective_sort;
#[path = "solar_system/scenes.rs"]
pub mod solar_system;
#[path = "test/scenes.rs"]
pub mod test;
#[path = "tetrahedron_spheres/scenes.rs"]
pub mod tetrahedron_spheres;
#[path = "text_item/scenes.rs"]
pub mod text_item;
#[path = "typst_timer/scenes.rs"]
pub mod typst_timer;

pub struct ExamplesPlugin;

impl ranim::Plugin for ExamplesPlugin {
    fn build(&self, app: &mut ranim::App) {
        app.add_plugins((
            aabb::ScenesPlugin,
            animating_pi::ScenesPlugin,
            arc::ScenesPlugin,
            arc_between_points::ScenesPlugin,
            basic::ScenesPlugin,
            bubble_sort::ScenesPlugin,
            cloth_wrap::ScenesPlugin,
            composable_choreography::ScenesPlugin,
            ellipse::ScenesPlugin,
            extract_vitem_visualize::ScenesPlugin,
            getting_started0::ScenesPlugin,
            getting_started1::ScenesPlugin,
            getting_started2::ScenesPlugin,
            hanoi::ScenesPlugin,
            hello_ranim::ScenesPlugin,
            iterative_spring::ScenesPlugin,
        ));
        app.add_plugins((
            mesh_morph::ScenesPlugin,
            midi_visualizer::ScenesPlugin,
            nbody::ScenesPlugin,
            output_formats::ScenesPlugin,
            palettes::ScenesPlugin,
            perlin_terrain::ScenesPlugin,
            perspective_blend::ScenesPlugin,
            ranim_logo::ScenesPlugin,
            regular_polygon::ScenesPlugin,
            selective_sort::ScenesPlugin,
            solar_system::ScenesPlugin,
            test::ScenesPlugin,
            tetrahedron_spheres::ScenesPlugin,
            text_item::ScenesPlugin,
            typst_timer::ScenesPlugin,
        ));
    }
}

// Export one native entry; hosts own the App and preview.
#[ranim::export_plugin]
pub fn plugin() -> impl ranim::Plugin {
    ExamplesPlugin
}
