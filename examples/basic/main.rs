use glam::DVec3;
use log::LevelFilter;
use ranim::{
    animation::{creation::WritingAnim, fading::FadingAnim, GroupAnimFunction},
    components::ScaleHint,
    items::{group::Group, vitem::VItem, GroupLaggedAnim},
    prelude::*,
    timeline::TimeMark,
    typst_svg,
    utils::rate_functions::linear,
};

const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[scene]
struct BasicScene;

impl TimelineConstructor for BasicScene {
    fn construct(self, timeline: &RanimTimeline, _camera: PinnedItem<CameraFrame>) {
        timeline.forward(0.2);

        let svg = Group::<VItem>::from_svg(SVG).with(|svg| {
            svg.scale_to_with_stroke(ScaleHint::PorportionalY(3.0))
                .put_center_on(DVec3::Y * 2.0);
        });

        let text = Group::<VItem>::from_svg(&typst_svg!(
            r#"
            #align(center)[
                #text(18pt)[Ranim]

                #text(6pt)[Hello 你好]
            ]
            "#
        ))
        .with(|text| {
            text.scale_to_with_stroke(ScaleHint::PorportionalY(2.0))
                .put_center_on(DVec3::NEG_Y * 2.0)
                .set_fill_opacity(0.8);
        });

        timeline.play(
            text.clone()
                .lagged_anim(0.2, |item| item.write())
                .with_total_duration(3.0)
                .with_rate_func(linear),
        );
        let text = timeline.pin(text);
        timeline.play(
            svg.clone()
                .lagged_anim(0.0, |item| item.fade_in().with_duration(3.0)),
        ); // At the same time, the svg fade in
        let svg = timeline.pin(svg);
        timeline.insert_time_mark(
            timeline.cur_sec(),
            TimeMark::Capture("preview.png".to_string()),
        );

        timeline.forward(0.5);
        timeline.play(
            timeline
                .unpin(text)
                .lagged_anim(0.2, |item| item.unwrite())
                .with_total_duration(3.0)
                .with_rate_func(linear),
        );
        timeline.play(
            timeline
                .unpin(svg)
                .lagged_anim(0.0, |item| item.fade_out().with_duration(3.0)),
        ); // At the same time, the svg fade in
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
