use ranim::glam;
use ranim_core::core_item::CoreItem;
use std::f64::consts::PI;

use ranim::{
    anims::{creation::WritingAnim, fading::FadingAnim, transform::TransformAnim},
    color,
    color::palettes::manim,
    core::{Extract, components::width::Width},
    items::vitem::{
        Group, VItem,
        geometry::{Circle, Square},
        svg::SvgItem,
        typst::typst_svg,
    },
    prelude::*,
};

use glam::DVec3;

// MARK: ranim_text
#[scene]
#[output(dir = "extract_vitem_visualize")]
fn ranim_text(r: &mut RanimScene) {
    let mut cam = CameraFrame::default();
    let r_cam = r.insert(cam.clone());

    let text = SvgItem::new(typst_svg("Ranim")).with(|item| {
        item.set_fill_color(manim::WHITE)
            .set_fill_opacity(0.5)
            .scale_to_with_stroke(ScaleHint::PorportionalY(3.6))
            .put_center_on(DVec3::ZERO);
    });
    let _r_texts = Group::<VItem>::from(text)
        .into_iter()
        .map(VisualVItem)
        .map(|item| r.insert(item))
        .collect::<Vec<_>>();

    r.timelines_mut().forward(1.0);
    r.timeline_mut(r_cam).play(cam.transform(|cam| {
        cam.scale = 0.3;
        cam.up = DVec3::NEG_X;
        cam.pos.shift(DVec3::NEG_X * 6.0);
    }));
    r.timelines_mut().forward(1.0);
    r.timeline_mut(r_cam).play(
        cam.transform(|cam| {
            cam.pos.shift(DVec3::X * 12.0);
        })
        .with_duration(7.0),
    );
    r.timelines_mut().forward(1.0);
    r.timeline_mut(r_cam)
        .play(cam.transform_to(CameraFrame::default()));

    // r.timelines_mut().forward(1.0);
    r.insert_time_mark(5.0, TimeMark::Capture("preview-ranim_text.png".to_string()));
}

#[scene(name = "extract_vitem_visualize")]
#[output(dir = "extract_vitem_visualize")]
pub fn hello_ranim(r: &mut RanimScene) {
    let _r_cam = r.insert(CameraFrame::default());

    let square = VisualVItem(VItem::from(Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    })));
    let mut circle = VisualVItem(VItem::from(Circle::new(2.0).with(|circle| {
        circle
            .set_color(manim::GREEN_C)
            .rotate(-PI / 4.0 + PI, DVec3::Z);
    })));

    let r_vitem = r.new_timeline();
    {
        let timeline = r.timeline_mut(r_vitem);
        timeline
            .play(square.clone().transform_to(circle.clone()))
            .forward(1.0);
        timeline
            .play(circle.clone().unwrite().with_duration(2.0))
            .play(circle.write().with_duration(2.0))
            .play(circle.fade_out());
    }

    r.timelines_mut().sync();
    r.insert_time_mark(
        3.2,
        TimeMark::Capture("preview-hello_ranim.png".to_string()),
    );
}

#[derive(Clone)]
pub struct VisualVItem(VItem);

impl Interpolatable for VisualVItem {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self(self.0.lerp(&target.0, t))
    }
}

impl Alignable for VisualVItem {
    fn is_aligned(&self, other: &Self) -> bool {
        self.0.is_aligned(&other.0)
    }
    fn align_with(&mut self, other: &mut Self) {
        self.0.align_with(&mut other.0);
    }
}

impl Partial for VisualVItem {
    fn get_partial(&self, range: std::ops::Range<f64>) -> Self {
        Self(self.0.get_partial(range))
    }
    fn get_partial_closed(&self, range: std::ops::Range<f64>) -> Self {
        Self(self.0.get_partial_closed(range))
    }
}

impl Opacity for VisualVItem {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.0.set_opacity(opacity);
        self
    }
}

impl StrokeColor for VisualVItem {
    fn set_stroke_color(&mut self, color: color::AlphaColor<color::Srgb>) -> &mut Self {
        self.0.set_stroke_color(color);
        self
    }
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.0.set_stroke_opacity(opacity);
        self
    }
    fn stroke_color(&self) -> color::AlphaColor<color::Srgb> {
        self.0.stroke_color()
    }
}

impl FillColor for VisualVItem {
    fn set_fill_color(&mut self, color: color::AlphaColor<color::Srgb>) -> &mut Self {
        self.0.set_fill_color(color);
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.0.set_fill_opacity(opacity);
        self
    }
    fn fill_color(&self) -> color::AlphaColor<color::Srgb> {
        self.0.fill_color()
    }
}

impl StrokeWidth for VisualVItem {
    fn stroke_width(&self) -> f32 {
        self.0.stroke_width()
    }
    fn apply_stroke_func(&mut self, f: impl for<'a> Fn(&'a mut [Width])) -> &mut Self {
        self.0.apply_stroke_func(f);
        self
    }
    fn set_stroke_width(&mut self, width: f32) -> &mut Self {
        self.0.set_stroke_width(width);
        self
    }
}

impl Extract for VisualVItem {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        // The VItem itself
        self.0.extract_into(buf);

        // VItem's vpoints as circles
        self.0.vpoints.get_subpaths().iter().for_each(|subpath| {
            let subpath_len = subpath.len();

            subpath.iter().enumerate().for_each(|(idx, p)| {
                if idx == subpath_len - 1 && !idx.is_multiple_of(2) {
                    return;
                }
                let point = if idx.is_multiple_of(2) {
                    Circle::new(0.06).with(|circle| {
                        circle
                            .set_color(if idx == 0 {
                                manim::GREEN_C
                            } else if idx / 2 == (subpath_len - 1) / 2 {
                                manim::RED_C
                            } else {
                                manim::BLUE_C
                            })
                            .set_fill_opacity(0.6);
                    })
                } else {
                    Circle::new(0.04).with(|circle| {
                        circle
                            .set_color(manim::WHITE)
                            .set_stroke_opacity(0.8)
                            .set_fill_opacity(0.4);
                    })
                }
                .with(|circle| {
                    circle.put_center_on(*p);
                });
                point.extract_into(buf);
            });
        });
        // lines between VItem's vpoints
        self.0
            .vpoints
            .iter()
            .step_by(2)
            .zip(self.0.vpoints.iter().skip(1).step_by(2))
            .zip(self.0.vpoints.iter().skip(2).step_by(2))
            .for_each(|((p0, p1), p2)| {
                if p0 != p1 {
                    VItem::from_vpoints(vec![*p0, (p0 + p1) / 2.0, *p1])
                        .with(|x| {
                            x.set_stroke_width(0.015);
                        })
                        .extract_into(buf);
                    VItem::from_vpoints(vec![*p1, (p1 + p2) / 2.0, *p2])
                        .with(|x| {
                            x.set_stroke_width(0.015);
                        })
                        .extract_into(buf);
                }
            });
    }
}

impl Empty for VisualVItem {
    fn empty() -> Self {
        Self(VItem::empty())
    }
}
