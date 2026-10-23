#![cfg_attr(target_arch = "wasm32", no_main)]
#![cfg(not(target_arch = "wasm32"))]

mod scenes;

fn main() {
    use ranim::prelude::*;
    let mut app = App::new();
    app.add_plugins(scenes::ScenesPlugin);
    run_cli(app.build());
}
