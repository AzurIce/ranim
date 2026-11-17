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
#[wasm_demo_doc]
#[output(dir = "./output/hello_ranim")]
fn hello_ranim(r: &mut RanimScene) {
    let square = Square::new(2.0).with(|x| {
        x.set_color(manim::BLUE_C);
    });

    let circle = Circle::new(2.0).with(|x| {
        x.set_color(manim::RED_C);
    });

    let mut content = AnimSequence::new();

    let mut vitem = VItem::from(square.clone());
    content
        .push(square.anim_ref(FadeIn).with_rate_func(smooth))
        .push(
            vitem
                .anim_mut(Morph::to(VItem::from(circle)))
                .with_rate_func(smooth),
        )
        .hold(1.0)
        .extend(seq![
            vitem.anim_ref(Unwrite).with_rate_func(smooth),
            vitem.anim_mut(Write).with_rate_func(smooth),
            vitem.anim_mut(FadeOut).with_rate_func(smooth),
        ]);

    r.play(
        CameraFrame::default()
            .show()
            .with_duration(content.cursor_sec()),
    );
    r.play(content);

    r.insert_time_mark(3.7, TimeMark::Capture("preview.png".to_string()));
}

/// Register the scenes provided by this example.
pub struct ScenesPlugin;

impl ranim::Plugin for ScenesPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_scenes([hello_ranim::scene()]);
    }
}
