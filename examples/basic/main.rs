use glam::DVec3;
use log::LevelFilter;
use ranim::{
    animation::{AnimGroupFunction, creation::WritingAnim, fading::FadingAnim},
    components::ScaleHint,
    items::vitem::{VItem, svg::SvgItem},
    prelude::*,
    timeline::{TimeMark, TimelineEncoder},
    typst_svg,
    utils::rate_functions::linear,
};

const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[scene]
struct BasicScene;

impl TimelineConstructor for BasicScene {
    fn construct(self, timeline: &RanimTimeline, _camera: &mut TimelineEncoder<CameraFrame>) {
        _camera.forward(0.2);

        let svg = Vec::<VItem>::from(SvgItem::new(SVG).with(|svg| {
            svg.scale_to_with_stroke(ScaleHint::PorportionalY(3.0))
                .put_center_on(DVec3::Y * 2.0);
        }));

        let text = Vec::<VItem>::from(
            SvgItem::new(&typst_svg!(
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
            }),
        );

        let timelines = text
            .clone()
            .into_iter()
            .map(|item| item.write())
            .collect::<Vec<_>>()
            .with_lagged_offset(0.2)
            .with_epilogue_to_end()
            .with_rate_func(linear)
            .into_iter()
            .map(|anim| {
                let mut timeline = _camera.new_timeline_encoder();
                let res = timeline.play(anim);
                (timeline, res)
            })
            .collect::<Vec<_>>();

        let svg_timeline = svg.clone().into_iter().map(|item| {
            let mut timeline = _camera.new_timeline_encoder();
            let res = timeline.play(item.fade_in().with_duration(3.0));
            (timeline, res)
        }).collect::<Vec<_>>();

        // At the same time, the svg fade in
        timeline.insert_time_mark(
            timeline.max_elapsed_secs(),
            TimeMark::Capture("preview.png".to_string()),
        );

        timeline.forward(0.5);
        timeline.play(
            timeline
                .unpin(text)
                .into_iter()
                .map(|item| item.unwrite())
                .collect::<Vec<_>>()
                .with_lagged_offset(0.2)
                .with_epilogue_to_end()
                .with_total_duration(3.0)
                .with_rate_func(linear),
        );
        timeline.play(
            timeline
                .unpin(svg)
                .into_iter()
                .map(|item| item.fade_out().with_duration(3.0))
                .collect::<Vec<_>>(),
        ); // At the same time, the svg fade in
        timeline.seal();
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
