use ranim::{
    anims::{creation::WritingAnim, morph::MorphAnim},
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Circle, Square},
    },
    prelude::*,
};

// ANCHOR: construct
#[scene]
#[output(dir = "getting_started1")]
fn getting_started1(r: &mut RanimScene) {
    let _r_cam = r.insert(CameraFrame::default());
    // A Square with size 2.0 and color blue
    let square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });

    let circle = Circle::new(2.0).with(|circle| {
        circle.set_color(manim::RED_C);
    });

    let r_vitem = r.insert_empty();
    {
        let timeline = r.timeline_mut(r_vitem);
        // In order to do more low-level opeerations,
        // sometimes we need to convert the item to a low-level item.
        timeline.play(VItem::from(square).morph_to(VItem::from(circle.clone())));
        timeline.play(VItem::from(circle).unwrite());
    }
}
// ANCHOR_END: construct
