fn main() {
    use ranim::{cmd, prelude::*};
    let mut app = App::new();
    app.add_plugins(ranim_render_plugin_example::ProjectPlugin);
    cmd::run_cli(app.build()).unwrap();
}
