use std::f64::consts::PI;

use ranim::{
    anims::{
        creation::{Unwrite, Write},
        fading::{FadeIn, FadeOut},
        morph::Morph,
    },
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Circle, Square},
    },
    prelude::*,
    utils::rate_functions::smooth,
};

#[scene]
pub fn hello_ranim(r: &mut RanimScene) {
    let mut square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });

    let mut content = seq![square.anim_mut(FadeIn).with_rate_func(smooth)];

    let mut circle: VItem = Circle::new(2.0)
        .with(|circle| {
            circle.set_color(manim::RED_C);
        })
        .into();
    circle.with_origin(AabbPoint::CENTER, |x| {
        x.rotate_on_z(PI / 4.0 + PI);
    });

    let mut vitem = VItem::from(square);
    content
        .push(vitem.anim_mut(Morph::to(circle)).with_rate_func(smooth))
        .hold(1.0)
        .push(vitem.anim_ref(Unwrite).with_rate_func(smooth))
        .push(vitem.anim_mut(Write).with_rate_func(smooth))
        .push(vitem.anim_mut(FadeOut).with_rate_func(smooth));

    r.play(
        CameraFrame::default()
            .show()
            .with_duration(content.cursor_sec()),
    );
    r.play(content);
}

pub struct ScenesPlugin;

impl ranim::Plugin for ScenesPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_scenes([hello_ranim::scene()]);
    }
}
