//! Examples to get started with ranim
use ranim::{
    animation::{
        creation::{CreationAnim, WritingAnim},
        fading::FadingAnim,
        transform::TransformAnim,
    },
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Circle, Rectangle, Square},
    },
    prelude::*,
    utils::rate_functions::linear,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[scene]
#[wasm_demo_doc]
#[preview]
#[output(dir = "getting_started0")]
/// This example shows the basic api of [`RanimScene`].
pub fn getting_started0(r: &mut RanimScene) {
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

#[scene]
#[wasm_demo_doc]
#[preview]
#[output(dir = "getting_started1")]
/// This example shows the basic api of [`RanimScene`].
pub fn getting_started1(r: &mut RanimScene) {
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

#[scene]
#[wasm_demo_doc]
#[preview]
#[output(dir = "getting_started2")]
/// This example shows the basic api of [`RanimScene`].
fn getting_started2(r: &mut RanimScene) {
    let _r_cam = r.insert_and_show(CameraFrame::default());
    let rect = Rectangle::new(4.0, 9.0 / 4.0).with(|rect| {
        rect.set_stroke_color(manim::GREEN_C);
    });

    // The new initialized timeline is hidden by default, use show to start encoding a static anim and make it show
    let r_rect: ItemId<Rectangle> = r.insert_and(rect, |timeline| {
        timeline.show();
    });
    // or use `insert_and_show`
    // let r_rect: ItemId<Rectangle> = r.insert_and_show(rect)

    r.timelines_mut().forward(1.0);

    let square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });
    let circle = Circle::new(2.0).with(|circle| {
        circle.set_color(manim::RED_C);
    });
    let r_vitem = r.insert(VItem::from(square));
    {
        let timeline = r.timeline_mut(&r_vitem);
        timeline
            .forward(1.0)
            .play_with(|vitem| vitem.create())
            .play_with(|vitem| {
                vitem
                    .transform_to(VItem::from(circle.clone()))
                    .with_rate_func(linear)
            })
            .play_with(|vitem| vitem.unwrite());
    }

    let r_rect: ItemId<VItem> = r.map(r_rect, VItem::from);
    r.timeline_mut(&r_rect).play_with(|rect| rect.uncreate());
}
