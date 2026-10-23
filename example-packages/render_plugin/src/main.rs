fn main() {
    use ranim::prelude::*;
    let mut app = App::new();
    app.add_plugins(ranim_render_plugin_example::ProjectPlugin);
    run_cli(app.build());
}
