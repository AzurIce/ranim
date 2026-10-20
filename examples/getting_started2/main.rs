mod scenes;

fn main() {
    use ranim::prelude::*;
    App::new().add_plugins(scenes::ScenesPlugin).run();
}
