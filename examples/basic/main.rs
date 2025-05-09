use glam::DVec3;
use log::LevelFilter;
use ranim::animation::creation::WritingAnimSchedule;
use ranim::animation::fading::FadingAnimSchedule;
use ranim::components::ScaleHint;
use ranim::items::group::Group;
use ranim::items::vitem::VItem;
use ranim::timeline::TimeMark;
use ranim::utils::rate_functions::linear;
use ranim::{prelude::*, typst_svg};

const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[scene]
struct BasicScene;

impl TimelineConstructor for BasicScene {
    fn construct(self, timeline: &RanimTimeline, _camera: &mut Rabject<CameraFrame>) {
        timeline.forward(0.2);

        let mut svg = Group::<VItem>::from_svg(SVG);
        svg.scale_to_with_stroke(ScaleHint::PorportionalHeight(3.0))
            .put_center_on(DVec3::Y * 2.0);
        let mut svg = timeline.insert(svg);

        let mut text = Group::<VItem>::from_svg(&typst_svg!(
            r#"
            #align(center)[
                #text(18pt)[Ranim]

                #text(6pt)[Hello 你好]
            ]
            "#
        ));
        text.scale_to_with_stroke(ScaleHint::PorportionalHeight(2.0))
            .put_center_on(DVec3::NEG_Y * 2.0);

        text.iter_mut().for_each(|item| {
            item.set_fill_opacity(0.8);
        });
        let mut text = timeline.insert(text);

        timeline.play(
            text.lagged_anim(0.2, |item| item.write())
                .with_total_duration(3.0)
                .with_rate_func(linear),
        );
        timeline.play(svg.lagged_anim(0.0, |item| item.fade_in().with_duration(3.0))); // At the same time, the svg fade in
        timeline.sync();
        timeline.insert_time_mark(
            timeline.duration_secs(),
            TimeMark::Capture("preview.png".to_string()),
        );

        timeline.forward(0.5);
        timeline.play(
            text.lagged_anim(0.2, |item| item.unwrite())
                .with_total_duration(3.0)
                .with_rate_func(linear),
        );
        timeline.play(svg.lagged_anim(0.0, |item| item.fade_out().with_duration(3.0))); // At the same time, the svg fade in
        timeline.sync();
    }
}

fn main() {
    #[cfg(debug_assertions)]
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("ranim"), LevelFilter::Trace)
        .init();
    #[cfg(not(debug_assertions))]
    pretty_env_logger::formatted_timed_builder()
        .filter(Some("ranim"), LevelFilter::Info)
        .init();
    render_scene(BasicScene, &AppOptions::default());
}
