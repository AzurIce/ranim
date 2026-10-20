fn main() {
    use ranim::prelude::*;
    App::new()
        .add_plugins(ranim_render_plugin_example::ProjectPlugin)
        .run();
}
