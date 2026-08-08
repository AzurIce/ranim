use std::f64::consts::PI;

use ranim::{
    anims::pure::{creation::WritingAnim, fading::FadingAnim, morph::MorphAnim},
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Circle, Square},
    },
    prelude::*,
    utils::rate_functions::smooth,
};

#[scene]
#[output(dir = "./output/hello_ranim")]
fn hello_ranim(r: &mut RanimScene) {
    let mut square = Square::new(2.0);
    square.set_color(manim::BLUE_C);

    let mut content = seq![square.clone().fade_in().with_rate_func(smooth)];

    let mut circle = Circle::new(2.0);
    circle
        .set_color(manim::RED_C)
        .with_origin(AabbPoint::CENTER, |x| {
            x.rotate_on_z(PI / 4.0 - PI);
        });

    let mut vitem = VItem::from(square);

    content.extend(seq![
        vitem.morph_to(circle.into()).with_rate_func(smooth),
        vitem.show(),
        vitem.clone().unwrite().with_rate_func(smooth),
        vitem.write().with_rate_func(smooth),
        vitem.fade_out().with_rate_func(smooth),
    ]);
    r.play(
        CameraFrame::default()
            .show()
            .with_duration(content.cursor_sec()),
    );
    r.play(content);

    r.insert_time_mark(3.7, TimeMark::Capture("preview.png".to_string()));
}

#[allow(unused)]
fn hello_ranim_chained(r: &mut RanimScene) {
    let square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });

    let mut content = seq![square.clone().fade_in().with_rate_func(smooth)];

    let circle = Circle::new(2.0).with(|circle| {
        circle
            .set_color(manim::RED_C)
            .with_origin(AabbPoint::CENTER, |x| {
                x.rotate_on_z(-PI / 4.0 + PI);
            });
    });

    let mut vitem = VItem::from(square);
    content
        .push(vitem.morph_to(circle.into()).with_rate_func(smooth))
        .hold(1.0)
        .extend(seq![
            vitem.clone().unwrite().with_rate_func(smooth),
            vitem.write().with_rate_func(smooth),
            vitem.fade_out().with_rate_func(smooth),
        ]);

    r.play(
        CameraFrame::default()
            .show()
            .with_duration(content.cursor_sec()),
    );
    r.play(content);

    r.insert_time_mark(3.7, TimeMark::Capture("preview.png".to_string()));
}
