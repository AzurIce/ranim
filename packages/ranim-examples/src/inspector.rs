//! Browse every bundled example without combining their renderer configurations.

#[cfg(not(target_family = "wasm"))]
use ranim::{App, AppExt, Plugin, RanimAppExt};
#[cfg(not(target_family = "wasm"))]
use ranim_examples::*;

#[cfg(not(target_family = "wasm"))]
fn project() -> ranim::RanimApp {
    let mut app = App::new();

    // Some examples register identically named scenes or mutually exclusive
    // render setups. Keep their assembly isolated, then collect only scene
    // descriptors under example-qualified names for this read-only browser.
    fn add<P: Plugin>(app: &mut App, example: &str, plugin: P) {
        let mut source = App::new();
        source.add_plugins(plugin);
        let source = source.build();
        for mut scene in source.scenes().iter().cloned() {
            scene.name = format!("{example} / {}", scene.name);
            app.add_scene(scene);
        }
    }

    macro_rules! examples {
        ($($example:ident),* $(,)?) => {
            $(add(&mut app, stringify!($example), $example::ScenesPlugin);)*
        };
    }
    examples!(
        aabb,
        animating_pi,
        arc,
        arc_between_points,
        audio_demo,
        basic,
        bubble_sort,
        cloth_wrap,
        composable_choreography,
        ellipse,
        extract_vitem_visualize,
        getting_started0,
        getting_started1,
        getting_started2,
        gltf_showcase,
        hanoi,
        hello_ranim,
        iterative_spring,
        mesh_morph,
        midi_visualizer,
        nbody,
        output_formats,
        palettes,
        perlin_terrain,
        perspective_blend,
        ranim_logo,
        regular_polygon,
        selective_sort,
        solar_system,
        test,
        tetrahedron_spheres,
        text_item,
        typst_timer,
        z_fighting,
    );
    app.build()
}

#[cfg(not(target_family = "wasm"))]
fn main() -> anyhow::Result<()> {
    ranim::cmd::cli::init_tracing();
    ranim::cmd::app::run_app(project())
}

#[cfg(target_family = "wasm")]
fn main() {}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;

    #[test]
    fn every_example_is_listed_without_building_scenes() {
        let app = project();
        assert!(app.scenes().len() >= 40);
        assert!(app.scene("basic / basic").is_some());
        assert!(
            app.scene("extract_vitem_visualize / extract_vitem_visualize")
                .is_some()
        );
        assert!(app.scene("hello_ranim / hello_ranim").is_some());
    }
}
