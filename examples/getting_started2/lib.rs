use ranim::{
    anims::{
        creation::{CreationAnim, WritingAnim},
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

#[scene]
#[output(dir = "getting_started2")]
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
