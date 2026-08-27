use std::f64::consts::{PI, TAU};

use ranim::{
    anims::camera::CameraFrameAnim,
    color::palettes::manim,
    glam::DVec3,
    items::{
        hierarchy::Node,
        mesh::{MeshItem, gltf::node_tree_from_path},
    },
    prelude::*,
    utils::rate_functions::linear,
};

const MODEL_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models");

/// Loads a model (the loader already converts glTF's Y-up to ranim's Z-up),
/// centers it on its own axes and normalizes its size, so it can be posed
/// by rotating about its vertical axis and placed by shifting its root.
fn load_model(file: &str, size: f64) -> Node<MeshItem> {
    let mut tree = node_tree_from_path(format!("{MODEL_DIR}/{file}"))
        .unwrap_or_else(|error| panic!("failed to load {file}: {error}"))
        .tree;
    let [min, max] = tree.aabb();
    let extents = max - min;
    tree.shift(-(min + max) / 2.0);
    tree.scale_uniform(size / extents.x.max(extents.y).max(extents.z));
    tree
}

/// Rotating showcase of the three dimension-factory machines. Each model is
/// a `Node<MeshItem>` tree; the spin is a per-frame root pose on the tree —
/// no vertex data is ever rewritten, and extraction composes the matrices
/// just like any other frame.
///
/// The POC glTF loader ignores materials, so each machine is tinted with a
/// palette color instead.
#[scene]
#[output(dir = "./output/gltf_showcase")]
fn gltf_showcase(r: &mut RanimScene) {
    let total_secs = 10.0;
    let fade_secs = 1.0;
    let spin_revolutions = 2.0;

    let mut cam = CameraFrame::from_spherical(65.0 * PI / 180.0, 25.0 * PI / 180.0, 9.5);
    cam.fovy = 40.0 * PI / 180.0;
    r.play(
        cam.orbit(DVec3::ZERO, PI / 5.0)
            .with_duration(total_secs)
            .with_rate_func(linear),
    );

    let slots = [-3.6, 0.0, 3.6];
    let colors = [manim::BLUE_D, manim::TEAL_D, manim::PURPLE_D];
    let files = ["belt.glb", "storage.glb", "generator.glb"];

    for (i, (file, &slot)) in files.iter().zip(slots.iter()).enumerate() {
        let mut tree = load_model(file, 2.2);
        tree.set_fill_color(colors[i]);

        // Spin about the model's own vertical axis while fading in over the
        // first second. The closure receives the segment's normalized alpha,
        // so durations are derived from total_secs. Everything is per-frame
        // data on the tree: local geometry stays constant throughout.
        let phase = i as f64 * TAU / 3.0;
        r.play(
            Pure::new(move |alpha| {
                let mut model = tree.clone();
                model.set_fill_opacity(((alpha * total_secs / fade_secs) as f32).min(1.0));
                model.rotate_on_axis(DVec3::Z, alpha * TAU * spin_revolutions + phase);
                model.shift(DVec3::X * slot);
                model
            })
            .with_duration(total_secs),
        );
    }

    r.insert_time_mark(2.0, TimeMark::Capture("preview_models.png".to_string()));
}
