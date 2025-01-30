use std::path::Path;

use bevy_color::Alpha;
use glam::{vec3, vec4};

use crate::{prelude::{Fill, Stroke}, utils::bezier::PathBuilder};

use super::vitem::VItem;

pub struct SvgItem {
    vitems: Vec<VItem>,
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
        for path in walk_svg_group(tree.root()) {
            let transform = path.abs_transform();

            let mut builder = PathBuilder::new();
            for segments in path.data().segments() {
                match segments {
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

            let mut vitem = VItem::from_vpoints(builder.vpoints().to_vec());
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
            }
            vitems.push(vitem);
        }

        Self { vitems }
    }
}

fn parse_paint(paint: &usvg::Paint) -> bevy_color::Srgba {
    match paint {
        usvg::Paint::Color(color) => {
            bevy_color::Color::srgb_u8(color.red, color.green, color.blue).to_srgba()
        }
        _ => bevy_color::Srgba {
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            alpha: 0.0,
        },
    }
}

struct SvgElementIterator<'a> {
    stack: Vec<&'a usvg::Group>,
    // transform_stack: Vec<usvg::Transform>,
}

impl<'a> Iterator for SvgElementIterator<'a> {
    type Item = &'a usvg::Path;
    fn next(&mut self) -> Option<Self::Item> {
        for node in self.stack.last().unwrap().children() {
            match node {
                usvg::Node::Group(group) => {
                    // self.transform_stack.push(group.abs_transform());
                    self.stack.push(&group);
                    return self.next();
                }
                usvg::Node::Path(path) => {
                    return Some(path);
                }
                usvg::Node::Image(image) => {}
                usvg::Node::Text(text) => {}
            }
        }
        None
    }
}

fn walk_svg_group(group: &usvg::Group) -> impl Iterator<Item = &usvg::Path> {
    SvgElementIterator { stack: vec![group] }
}
