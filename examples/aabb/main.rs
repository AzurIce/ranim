mod scenes;

/// Registers this project's scenes and rendering support.
pub struct ProjectPlugin;

impl ranim::Plugin for ProjectPlugin {
    fn build(&self, app: &mut ranim::App) {
        app.add_plugins(scenes::ScenesPlugin);

        use ranim::{
            items::{RanimItemsPlugin, debug::VisualizeAabbItem, vitem::VItem},
            render::{RenderSetup, extract::ExtractPlugin},
        };

        app.insert_resource(RenderSetup::new(|render_app| {
            render_app.add_plugins((
                RanimItemsPlugin,
                ExtractPlugin::<VisualizeAabbItem<VItem>>::default(),
            ));
        }));
    }
}

fn main() {
    use ranim::prelude::*;
    App::new().add_plugins(ProjectPlugin).run();
}
