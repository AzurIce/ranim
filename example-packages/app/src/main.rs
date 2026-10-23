fn main() {
    use ranim::prelude::*;
    let mut app = App::new();
    app.add_plugins(app::ScenesPlugin);
    run_cli(app.build());
}
