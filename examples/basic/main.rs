use glam::DVec3;
use log::LevelFilter;
use ranim::{
    animation::{creation::WritingAnim, fading::FadingAnim, lagged::LaggedAnim},
    color::palettes::manim,
    components::ScaleHint,
    items::{
        Group,
        vitem::{VItem, svg::SvgItem, typst::typst_svg},
    },
    prelude::*,
    timeline::TimeMark,
};

const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[scene]
struct BasicScene;

impl SceneConstructor for BasicScene {
    fn construct(self, r: &mut RanimScene, _r_cam: ItemId<CameraFrame>) {
        r.timelines_mut().forward(0.2);

        let svg = Group::<VItem>::from(SvgItem::new(SVG).with(|svg| {
            svg.scale_to_with_stroke(ScaleHint::PorportionalY(3.0))
                .put_center_on(DVec3::Y * 2.0);
        }));
        let text = Group::<VItem>::from(
            SvgItem::new(typst_svg(
                r#"
            #align(center)[
                #text(18pt)[Ranim]

                #text(6pt)[Hello 你好]
            ]
            "#,
            ))
            .with(|text| {
                text.scale_to_with_stroke(ScaleHint::PorportionalY(2.0))
                    .put_center_on(DVec3::NEG_Y * 2.0)
                    .set_color(manim::WHITE)
                    .set_fill_opacity(0.8);
            }),
        );
        let r_svg = r.insert(svg);
        let r_text = r.insert(text);

        r.timeline_mut(&r_text)
            .play_with(|text| text.lagged(0.2, |e| e.write()).with_duration(3.0));
        r.timeline_mut(&r_svg)
            .play_with(|svg| svg.fade_in().with_duration(3.0)); // At the same time, the svg fade in
        r.timelines_mut().sync();

        r.insert_time_mark(
            r.timelines().max_total_secs(),
            TimeMark::Capture("preview.png".to_string()),
        );

        r.timelines_mut().forward(0.5);
        r.timeline_mut(&r_text)
            .play_with(|text| text.lagged(0.2, |e| e.write()).with_duration(3.0));
        r.timeline_mut(&r_svg)
            .play_with(|svg| svg.fade_out().with_duration(3.0)); // At the same time, the svg fade out
        r.timelines_mut().sync();
    }
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        #[cfg(debug_assertions)]
        pretty_env_logger::formatted_timed_builder()
            .filter(Some("ranim"), LevelFilter::Trace)
            .init();
        #[cfg(not(debug_assertions))]
        pretty_env_logger::formatted_timed_builder()
            .filter(Some("ranim"), LevelFilter::Info)
            .init();
    }

    #[cfg(feature = "app")]
    run_scene_app(BasicScene);
    #[cfg(not(feature = "app"))]
    render_scene(BasicScene, &AppOptions::default());
}
