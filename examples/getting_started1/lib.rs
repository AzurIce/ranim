use ranim::{
    animation::{creation::WritingAnim, transform::TransformAnim},
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
    let _r_cam = r.insert_and_show(CameraFrame::default());
    // A Square with size 2.0 and color blue
    let square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });

    let circle = Circle::new(2.0).with(|circle| {
        circle.set_color(manim::RED_C);
    });

    // In order to do more low-level opeerations,
    // sometimes we need to convert the item to a low-level item.
    let r_vitem = r.insert(VItem::from(square));
    {
        let timeline = r.timeline_mut(&r_vitem);
        timeline.play_with(|vitem| vitem.transform_to(VItem::from(circle.clone())));
        timeline.play_with(|vitem| vitem.unwrite());
    }
}
// ANCHOR_END: construct
