#[cfg(target_arch = "wasm32")]
use ranim_app as _;

use ranim::{
    anims::fading::FadingAnim, color::palettes::manim, items::vitem::geometry::Square, prelude::*,
};

// ANCHOR: construct
#[scene]
#[output(dir = "getting_started0")]
fn getting_started0(r: &mut RanimScene) {
    // Equivalent to creating a new timeline then playing `CameraFrame::default().show()` on it
    let _r_cam = r.insert(CameraFrame::default());
    // A Square with size 2.0 and color blue
    let square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });

    let r_square = r.insert_empty();
    {
        let timeline = r.timeline_mut(r_square);
        timeline
            .play(square.clone().fade_in()) // Can be written as `square.fade_in_ref()`
            .forward(1.0)
            .hide()
            .forward(1.0)
            .show()
            .forward(1.0)
            .play(square.clone().fade_out()); // Can be written as `square.fade_out_ref()`
    }
    // In the end, ranim will automatically sync all timelines and forward to the end.
    // Equivalent to `r.timelines_mut().sync();`
}
// ANCHOR_END: construct
