use ranim::prelude::*;

use ranim::{
    cmd,
    color::palettes::manim,
    items::vitem::geometry::{Circle, Square},
};

#[scene]
#[output]
fn hello_ranim(r: &mut RanimScene) {
    let square = Square::new(2.0).with(|x| {
        x.set_color(manim::BLUE_C);
    });

    let circle = Circle::new(2.0).with(|x| {
        x.set_color(manim::RED_C);
    });

    r.play(square.show());
    r.play(circle.show().with_duration(2.0));

    r.play(
        CameraFrame::default()
            .show()
            .with_duration(r.duration_secs()),
    );
}

fn main() {
    let mut app = App::new();
    app.add_scene(hello_ranim::scene());
    cmd::run_cli(app.build()).unwrap();
}
