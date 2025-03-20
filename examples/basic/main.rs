use env_logger::Env;
use glam::Vec3;
use ranim::animation::creation::WritingAnimSchedule;
use ranim::animation::fading::FadingAnimSchedule;
use ranim::items::group::Group;
use ranim::items::svg_item::SvgItem;
use ranim::items::vitem::VItem;
use ranim::timeline::TimeMark;
use ranim::utils::rate_functions::linear;
use ranim::{prelude::*, typst_svg};

const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

#[scene]
struct BasicScene;

impl TimelineConstructor for BasicScene {
    fn construct<'t: 'r, 'r>(
        self,
        timeline: &'t RanimTimeline,
        _camera: &'r mut Rabject<'t, CameraFrame>,
    ) {
        timeline.forward(0.2);

        let mut svg = SvgItem::from_svg(SVG);
        {
            let bb = svg.get_bounding_box();
            svg.scale(Vec3::splat(3.0 / (bb[2].y - bb[0].y)))
                .put_center_on(Vec3::Y * 2.0);
        }
        let mut svg = timeline.insert(svg);

        let mut text = Group::<VItem>::from_svg(&typst_svg!(
            r#"
            #align(center)[
                #text(18pt)[Ranim]

                #text(6pt)[Hello 你好]
            ]
            "#
        ));
        {
            let bb = text.get_bounding_box();
            text.scale(Vec3::splat(2.0 / (bb[2].y - bb[0].y)))
                .put_center_on(Vec3::NEG_Y * 2.0);
        }

        text.iter_mut().for_each(|item| {
            item.set_fill_opacity(0.8);
        });
        let mut text = timeline.insert_group(text);

        timeline.play(
            text.lagged_anim(0.2, |item| item.write())
                .with_total_duration(3.0)
                .with_rate_func(linear),
        );
        timeline.play(svg.fade_in().with_duration(3.0)); // At the same time, the svg fade in
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
        timeline.play(svg.fade_out().with_duration(3.0));
        timeline.sync();
    }
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("basic=info,ranim=info")).init();

    render_scene(BasicScene, &AppOptions::default());
}
