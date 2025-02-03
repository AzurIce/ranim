use std::{f32, path::Path, slice::Iter, vec};

use bevy_color::Alpha;
use glam::{mat3, vec2, vec3, Vec3};
use itertools::Itertools;
use log::{info, trace, warn};

use crate::{
    components::{rgba::Rgba, width::Width, HasTransform3d, TransformAnchor},
    prelude::{Empty, Fill, Interpolatable, Partial, Stroke},
    render::primitives::{svg_item::SvgItemPrimitive, vitem::VItemPrimitive, Extract},
    utils::bezier::PathBuilder,
};

use super::{vitem::VItem, Entity};

#[derive(Debug, Clone)]
pub struct SvgItem {
    pub(crate) vitems: Vec<VItem>,
}

impl SvgItem {
    pub fn from_file(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).unwrap();
        Self::from_svg(&content)
    }
    pub fn from_svg(svg: impl AsRef<str>) -> Self {
        let svg = svg.as_ref();
        let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();

        let mut vitems = vec![];
        for (path, transform) in walk_svg_group(tree.root()) {
            // let transform = path.abs_transform();

            let mut builder = PathBuilder::new();
            for segment in path.data().segments() {
                match segment {
                    usvg::tiny_skia_path::PathSegment::MoveTo(p) => {
                        builder.move_to(vec3(p.x, p.y, 0.0))
                    }
                    usvg::tiny_skia_path::PathSegment::LineTo(p) => {
                        builder.line_to(vec3(p.x, p.y, 0.0))
                    }
                    usvg::tiny_skia_path::PathSegment::QuadTo(p1, p2) => {
                        builder.quad_to(vec3(p1.x, p1.y, 0.0), vec3(p2.x, p2.y, 0.0))
                    }
                    usvg::tiny_skia_path::PathSegment::CubicTo(p1, p2, p3) => builder.cubic_to(
                        vec3(p1.x, p1.y, 0.0),
                        vec3(p2.x, p2.y, 0.0),
                        vec3(p3.x, p3.y, 0.0),
                    ),
                    usvg::tiny_skia_path::PathSegment::Close => builder.close_path(),
                };
            }
            if builder.is_empty() {
                warn!("empty path");
                continue;
            }

            let mut vitem = VItem::from_vpoints(builder.vpoints().to_vec());
            vitem.vpoints.apply_affine(mat3(
                vec3(transform.sx, transform.kx, transform.tx),
                vec3(transform.ky, transform.sy, transform.ty),
                vec3(0.0, 0.0, 1.0),
            ));
            vitem
                .vpoints
                .rotate(f32::consts::PI, Vec3::X, TransformAnchor::origin());
            if let Some(fill) = path.fill() {
                let color = parse_paint(fill.paint()).with_alpha(fill.opacity().get());
                vitem.set_fill_color(color);
            } else {
                vitem.set_fill_color(bevy_color::Srgba {
                    red: 0.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 0.0,
                });
            }
            if let Some(stroke) = path.stroke() {
                let color = parse_paint(stroke.paint()).with_alpha(stroke.opacity().get());
                vitem.set_stroke_color(color);
                vitem.set_stroke_width(stroke.width().get());
            } else {
                vitem.set_stroke_color(bevy_color::Srgba {
                    red: 0.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 0.0,
                });
            }
            vitems.push(vitem);
        }
        // vitems.reverse();

        Self { vitems }
    }
}

impl Empty for SvgItem {
    fn empty() -> Self {
        Self {
            vitems: vec![VItem::empty()],
        }
    }
}

impl Entity for SvgItem {
    type Primitive = SvgItemPrimitive;
    fn clip_box(&self, camera: &crate::render::CameraFrame) -> [glam::Vec2; 4] {
        self.vitems
            .iter()
            .map(|x| x.clip_box(camera))
            .reduce(|acc, x| {
                [
                    vec2(acc[0].x.min(x[0].x), acc[0].y.min(x[0].y)),
                    vec2(acc[1].x.min(x[1].x), acc[1].y.max(x[1].y)),
                    vec2(acc[2].x.max(x[2].x), acc[2].y.min(x[2].y)),
                    vec2(acc[3].x.max(x[3].x), acc[3].y.max(x[3].y)),
                ]
            })
            .unwrap_or([
                vec2(-1.0, -1.0),
                vec2(-1.0, 1.0),
                vec2(1.0, -1.0),
                vec2(1.0, 1.0),
            ])
    }
}

// MARK: Extract impl

impl Extract<SvgItem> for SvgItemPrimitive {
    fn update(&mut self, ctx: &crate::context::WgpuContext, data: &SvgItem) {
        // trace!("SvgItemPrimitive update vitems: {}", data.vitems.len());
        if data.vitems.len() != self.vitem_primitives.len() {
            // trace!("resizing vitem_primitives from {} to {}...", self.vitem_primitives.len(), data.vitems.len());
            self.vitem_primitives
                .resize_with(data.vitems.len(), Default::default);
            self.refresh_clip_box(ctx);
        }
        // trace!("updating vitem_primitives...");
        self.vitem_primitives
            .iter_mut()
            .zip(data.vitems.iter())
            .for_each(|(vitem_primitive, vitem)| {
                Extract::<VItem>::update(vitem_primitive, ctx, vitem)
            });
        // let anchor_points_cnt = data
        //     .vitems
        //     .iter()
        //     .map(|x| (x.vpoints.len() + 1) / 2)
        //     .sum::<usize>();
        // let mut render_points = Vec::with_capacity(anchor_points_cnt * 2 - 1);
        // let mut fill_rgbas = Vec::with_capacity(anchor_points_cnt);
        // let mut stroke_rgbas = Vec::with_capacity(anchor_points_cnt);
        // let mut stroke_widths = Vec::with_capacity(anchor_points_cnt);

        // for vitem in &data.vitems {
        //     if vitem.vpoints.is_empty() {
        //         continue;
        //     }
        //     let data = vitem.get_render_points();
        //     render_points.extend_from_slice(&data);
        //     render_points.push(data.last().cloned().unwrap());

        //     let data = AsRef::<[Rgba]>::as_ref(&vitem.fill_rgbas);
        //     fill_rgbas.extend_from_slice(data);
        //     fill_rgbas.push(data.last().cloned().unwrap());

        //     let data = AsRef::<[Rgba]>::as_ref(&vitem.stroke_rgbas);
        //     stroke_rgbas.extend_from_slice(data);
        //     stroke_rgbas.push(data.last().cloned().unwrap());

        //     let data = AsRef::<[Width]>::as_ref(&vitem.stroke_widths);
        //     stroke_widths.extend_from_slice(data);
        //     stroke_widths.push(data.last().cloned().unwrap());
        // }

        // info!(
        //     "SvgItem({} vitems) Extract len: {} {} {} {}",
        //     data.vitems.len(),
        //     render_points.len(),
        //     fill_rgbas.len(),
        //     stroke_rgbas.len(),
        //     stroke_widths.len()
        // );
        // self.update(
        //     ctx,
        //     &render_points,
        //     &fill_rgbas,
        //     &stroke_rgbas,
        //     &stroke_widths,
        // );
    }
}

// MARK: Animation impl

impl Interpolatable for SvgItem {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        let vitems = self
            .vitems
            .iter()
            .zip(target.vitems.iter())
            .map(|(a, b)| a.lerp(b, t))
            .collect();
        Self { vitems }
    }
}

impl Partial for SvgItem {
    fn get_partial(&self, range: std::ops::Range<f32>) -> Self {
        let start = range.start * (self.vitems.len() - 1) as f32;
        let end = range.end * (self.vitems.len() - 1) as f32;

        let start_idx = start.floor();
        let end_idx = end.ceil();

        Self {
            vitems: self
                .vitems
                .get(start_idx as usize..=end_idx as usize)
                .unwrap()
                .to_owned(),
        }
    }
}

// MARK: misc
fn parse_paint(paint: &usvg::Paint) -> bevy_color::Srgba {
    match paint {
        usvg::Paint::Color(color) => {
            bevy_color::Color::srgb_u8(color.red, color.green, color.blue).to_srgba()
        }
        _ => bevy_color::Srgba {
            red: 0.0,
            green: 1.0,
            blue: 0.0,
            alpha: 1.0,
        },
    }
}

struct SvgElementIterator<'a> {
    // Group children iter and its transform
    stack: Vec<(Iter<'a, usvg::Node>, usvg::Transform)>,
    // transform_stack: Vec<usvg::Transform>,
}

impl<'a> Iterator for SvgElementIterator<'a> {
    type Item = (&'a usvg::Path, usvg::Transform);
    fn next(&mut self) -> Option<Self::Item> {
        while !self.stack.is_empty() {
            let (group, transform) = self.stack.last_mut().unwrap();
            match group.next() {
                Some(node) => match node {
                    usvg::Node::Group(group) => {
                        self.stack
                            .push((group.children().iter(), group.abs_transform()));
                    }
                    usvg::Node::Path(path) => {
                        return Some((path, *transform));
                    }
                    usvg::Node::Image(image) => {}
                    usvg::Node::Text(text) => {}
                },
                None => {
                    self.stack.pop();
                }
            }
            return self.next();
        }
        None
    }
}

fn walk_svg_group(group: &usvg::Group) -> impl Iterator<Item = (&usvg::Path, usvg::Transform)> {
    SvgElementIterator {
        stack: vec![(group.children().iter(), usvg::Transform::identity())],
    }
}

#[cfg(test)]
mod test {
    use super::walk_svg_group;

    const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");
    #[test]
    fn test_svg_element_iter() {
        let tree = usvg::Tree::from_str(SVG, &usvg::Options::default()).unwrap();
        let paths = walk_svg_group(tree.root()).collect::<Vec<_>>();
        println!("{} paths", paths.len());
    }
}
