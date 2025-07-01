use std::{slice::Iter, vec};

use color::{AlphaColor, Srgb, palette::css};
use glam::{DAffine2, dvec3};
use log::warn;

use crate::{
    color::{rgb8, rgba},
    items::vitem::VItem,
    prelude::{FillColor, StrokeWidth},
    traits::{PointsFunc, StrokeColor},
    utils::bezier::PathBuilder,
};

// MARK: misc
fn parse_paint(paint: &usvg::Paint) -> AlphaColor<Srgb> {
    match paint {
        usvg::Paint::Color(color) => rgb8(color.red, color.green, color.blue),
        _ => css::GREEN,
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
        #[allow(clippy::never_loop)]
        while !self.stack.is_empty() {
            let (group, transform) = self.stack.last_mut().unwrap();
            match group.next() {
                Some(node) => match node {
                    usvg::Node::Group(group) => {
                        // trace!("group {:?}", group.abs_transform());
                        self.stack
                            .push((group.children().iter(), group.abs_transform()));
                    }
                    usvg::Node::Path(path) => {
                        return Some((path, *transform));
                    }
                    usvg::Node::Image(_image) => {}
                    usvg::Node::Text(_text) => {}
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

pub fn vitems_from_tree(tree: &usvg::Tree) -> Vec<VItem> {
    let mut vitems = vec![];
    for (path, transform) in walk_svg_group(tree.root()) {
        // let transform = path.abs_transform();

        let mut builder = PathBuilder::new();
        for segment in path.data().segments() {
            match segment {
                usvg::tiny_skia_path::PathSegment::MoveTo(p) => {
                    builder.move_to(dvec3(p.x as f64, p.y as f64, 0.0))
                }
                usvg::tiny_skia_path::PathSegment::LineTo(p) => {
                    builder.line_to(dvec3(p.x as f64, p.y as f64, 0.0))
                }
                usvg::tiny_skia_path::PathSegment::QuadTo(p1, p2) => builder.quad_to(
                    dvec3(p1.x as f64, p1.y as f64, 0.0),
                    dvec3(p2.x as f64, p2.y as f64, 0.0),
                ),
                usvg::tiny_skia_path::PathSegment::CubicTo(p1, p2, p3) => builder.cubic_to(
                    dvec3(p1.x as f64, p1.y as f64, 0.0),
                    dvec3(p2.x as f64, p2.y as f64, 0.0),
                    dvec3(p3.x as f64, p3.y as f64, 0.0),
                ),
                usvg::tiny_skia_path::PathSegment::Close => builder.close_path(),
            };
        }
        if builder.is_empty() {
            warn!("empty path");
            continue;
        }

        let mut vitem = VItem::from_vpoints(builder.vpoints().to_vec());
        let affine = DAffine2::from_cols_array(&[
            transform.sx as f64,
            transform.kx as f64,
            transform.kx as f64,
            transform.sy as f64,
            transform.tx as f64,
            transform.ty as f64,
        ]);
        vitem.apply_affine(affine);
        if let Some(fill) = path.fill() {
            let color = parse_paint(fill.paint()).with_alpha(fill.opacity().get());
            vitem.set_fill_color(color);
        } else {
            vitem.set_fill_color(rgba(0.0, 0.0, 0.0, 0.0));
        }
        if let Some(stroke) = path.stroke() {
            let color = parse_paint(stroke.paint()).with_alpha(stroke.opacity().get());
            vitem.set_stroke_color(color);
            vitem.set_stroke_width(stroke.width().get());
        } else {
            vitem.set_stroke_color(rgba(0.0, 0.0, 0.0, 0.0));
        }
        vitems.push(vitem);
    }
    vitems
}

#[cfg(test)]
mod test {
    use super::*;

    const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");
    #[test]
    fn test_svg_element_iter() {
        let tree = usvg::Tree::from_str(SVG, &usvg::Options::default()).unwrap();
        let paths = walk_svg_group(tree.root()).collect::<Vec<_>>();
        println!("{} paths", paths.len());
    }

    // #[test]
    // fn test_get_partial() {
    //     let mut svg = SvgItem::from_svg(typst_svg!("R"));
    //     svg.scale(DVec3::splat(10.0));

    //     println!("{:?}", svg.vitems[0].vpoints);
    //     let partial = svg.get_partial(0.0..0.5);
    //     println!("{:?}", partial.vitems[0].vpoints);
    //     let partial = svg.vitems[0].get_partial(0.0..0.5);
    //     println!("{:?}", partial.vpoints);
    // }
}
