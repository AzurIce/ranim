//! A project with a private animation output type and its own render setup.

use ranim::{
    App, Plugin, RanimScene, Scene,
    color::palette::css,
    items::{
        RanimItemsPlugin,
        camera::CameraFrame,
        mesh::MeshItem,
        vitem::{VItem, geometry::Square},
    },
    prelude::{FillColor, PlaybackExt, ShiftTransform, Static, StrokeColor, StrokeWidth},
    render::{
        RenderSetup,
        extract::{Extract, ExtractContext, ExtractPlugin},
    },
};

/// The host cannot name this complete animation output type.
#[derive(Clone)]
struct PrivateDrawing {
    vector: VItem,
    mesh: MeshItem,
}

impl Extract for PrivateDrawing {
    fn extract(&self, ctx: &mut ExtractContext<'_>) {
        self.vector.extract(ctx);
        self.mesh.extract(ctx);
    }
}

fn scene(scene: &mut RanimScene) {
    use ranim::glam::DVec3;

    let mut vector = VItem::from(Square::new(1.6));
    vector
        .shift(DVec3::new(-1.5, 0.0, 0.0))
        .set_fill_color(css::RED)
        .set_fill_opacity(1.0)
        .set_stroke_color(css::RED)
        .set_stroke_width(0.04);
    let mesh = MeshItem::from_indexed_vertices(
        vec![
            DVec3::new(0.6, -0.8, 0.0),
            DVec3::new(2.4, -0.8, 0.0),
            DVec3::new(1.5, 0.9, 0.0),
        ],
        vec![0, 1, 2],
    )
    .with_color(css::BLUE);

    scene.play(Static(CameraFrame::default()).with_duration(1.0));
    scene.play(Static(PrivateDrawing { vector, mesh }).with_duration(1.0));
}

/// Configure the project's scene and rendering support.
pub struct ProjectPlugin;

impl Plugin for ProjectPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RenderSetup::new(|render_app| {
            render_app.add_plugins((RanimItemsPlugin, ExtractPlugin::<PrivateDrawing>::default()));
        }))
        .add_scene(Scene::new("render_plugin", scene));
    }
}
