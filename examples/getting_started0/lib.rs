use ranim::{
    animation::fading::FadingAnim, color::palettes::manim, items::vitem::geometry::Square,
    prelude::*,
};

#[scene]
#[preview]
#[output(dir = "getting_started0")]
fn getting_started0(r: &mut RanimScene) {
    let _r_cam = r.insert_and_show(CameraFrame::default());
    // A Square with size 2.0 and color blue
    let square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });

    let r_square = r.insert(square);
    {
        let timeline = r.timeline_mut(&r_square);
        timeline
            .play_with(|square| square.fade_in())
            .forward(1.0)
            .hide()
            .forward(1.0)
            .show()
            .forward(1.0)
            .play_with(|square| square.fade_out());
    }
}
// ANCHOR_END: construct
