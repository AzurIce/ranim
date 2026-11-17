fn main() {
    use ranim::{cmd, prelude::*};
    let mut app = App::new();
    app.add_plugins(app::ScenesPlugin);
    cmd::run_cli(app.build()).unwrap();
}
