use color::{AlphaColor, Srgb, palette::css, rgb8, rgba};
use glam::DVec3;
use glam::{DAffine2, dvec3};
use ranim_core::core_item::CoreItem;
use ranim_core::traits::PointsFunc;
use ranim_core::{
    Extract, components::width::Width, traits::AnchorPoint, utils::bezier::PathBuilder,
};
use ranim_core::{color, glam};
use tracing::warn;

use ranim_core::traits::{
    Aabb, FillColor, Opacity, Rotate, Scale, Shift, StrokeColor, StrokeWidth,
};

use super::VItem;

// MARK: ### SvgItem ###
/// An Svg Item
///
/// Its inner is a `Vec<VItem>`
#[derive(Clone)]
pub struct SvgItem(Vec<VItem>);

impl From<SvgItem> for Vec<VItem> {
    fn from(value: SvgItem) -> Self {
        value.0
    }
}

impl SvgItem {
    /// Creates a new SvgItem from a SVG string
    pub fn new(svg: impl AsRef<str>) -> Self {
        let mut vitem_group = Self(vitems_from_svg(svg.as_ref()));
        vitem_group.move_to(DVec3::ZERO);
        vitem_group.rotate(std::f64::consts::PI, DVec3::X);
        vitem_group
    }
}

// MARK: Trait impls
impl Aabb for SvgItem {
    fn aabb(&self) -> [glam::DVec3; 2] {
        self.0.aabb()
    }
}

impl Shift for SvgItem {
    fn shift(&mut self, shift: glam::DVec3) -> &mut Self {
        self.0.shift(shift);
        self
    }
}

impl Rotate for SvgItem {
    fn rotate_at(&mut self, angle: f64, axis: glam::DVec3, anchor: impl AnchorPoint) -> &mut Self {
        self.0.rotate_at(angle, axis, anchor);
        self
    }
}

impl Scale for SvgItem {
    fn scale_at(&mut self, scale: glam::DVec3, anchor: impl AnchorPoint) -> &mut Self {
        self.0.scale_at(scale, anchor);
        self
    }
}

impl FillColor for SvgItem {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.0[0].fill_color()
    }
    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.0.set_fill_color(color);
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.0.set_fill_opacity(opacity);
        self
    }
}

impl StrokeColor for SvgItem {
    fn stroke_color(&self) -> AlphaColor<Srgb> {
        self.0[0].fill_color()
    }
    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.0.set_stroke_color(color);
        self
    }
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.0.set_stroke_opacity(opacity);
        self
    }
}

impl Opacity for SvgItem {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.0.set_fill_opacity(opacity);
        self.0.set_stroke_opacity(opacity);
        self
    }
}

impl StrokeWidth for SvgItem {
    fn stroke_width(&self) -> f32 {
        self.0.stroke_width()
    }
    fn apply_stroke_func(&mut self, f: impl for<'a> Fn(&'a mut [Width])) -> &mut Self {
        self.0.iter_mut().for_each(|vitem| {
            vitem.apply_stroke_func(&f);
        });
        self
    }
    fn set_stroke_width(&mut self, width: f32) -> &mut Self {
        self.0.set_stroke_width(width);
        self
    }
}

// MARK: Conversions
impl Extract for SvgItem {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        self.0.extract_into(buf);
    }
}

// MARK: misc
fn parse_paint(paint: &usvg::Paint) -> AlphaColor<Srgb> {
    match paint {
        usvg::Paint::Color(color) => rgb8(color.red, color.green, color.blue),
        _ => css::GREEN,
    }
}

struct SvgElementIterator<'a> {
    // Group children iter and its transform
    stack: Vec<(std::slice::Iter<'a, usvg::Node>, usvg::Transform)>,
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

/// Construct a `Vec<VItem` from `&str` of a SVG
pub fn vitems_from_svg(svg: &str) -> Vec<VItem> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    vitems_from_tree(&tree)
}

/// Construct a `Vec<VItem>` from `&usvg::Tree`
pub fn vitems_from_tree(tree: &usvg::Tree) -> Vec<VItem> {
    let mut vitems = vec![];
    for (path, transform) in walk_svg_group(tree.root()) {
        // println!("path: {:?}", path);
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
        let fill_color = if let Some(fill) = path.fill() {
            parse_paint(fill.paint()).with_alpha(fill.opacity().get())
        } else {
            rgba(0.0, 0.0, 0.0, 0.0)
        };
        vitem.set_fill_color(fill_color);
        if let Some(stroke) = path.stroke() {
            let color = parse_paint(stroke.paint()).with_alpha(stroke.opacity().get());
            vitem.set_stroke_color(color);
            vitem.set_stroke_width(stroke.width().get());
        } else {
            vitem.set_stroke_color(fill_color.with_alpha(0.0));
            vitem.set_stroke_width(0.0);
        }
        vitems.push(vitem);
    }
    vitems
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use glam::dvec3;

    use crate::vitem::{geometry::Arc, typst::typst_svg};
    use ranim_core::traits::{AabbPoint, ScaleHint, ScaleStrokeExt, With};

    use super::*;
    #[test]
    fn foo_test_vitems_from_svg() {
        let svg = typst_svg("R");
        let mut vitems = vitems_from_svg(&svg);

        println!("{:?}", vitems.aabb());
        let scale = vitems.calc_scale_ratio(ScaleHint::PorportionalY(8.0));
        println!("scale: {}", scale);
        let center = AabbPoint::CENTER.get_pos(vitems.as_slice());
        println!("{:?}", center);
        vitems
            .scale_to(ScaleHint::PorportionalY(8.0))
            .move_to(DVec3::ZERO);

        println!("");
        println!(
            "{:?}",
            vitems.iter().map(|x| &x.vpoints).collect::<Vec<_>>()
        );
    }

    fn print_typst_vitem(points: Vec<DVec3>) {
        let colors = ["blue.darken(40%)", "yellow.darken(50%)"];
        let mut last_anchor = None;
        let mut subpath_cnt = 0;
        let segs = points
            .iter()
            .step_by(2)
            .cloned()
            .zip(points.iter().skip(1).step_by(2).cloned())
            .zip(points.iter().skip(2).step_by(2).cloned())
            .collect::<Vec<_>>();

        segs.iter().enumerate().for_each(|(i, ((a, b), c))| {
            if last_anchor.is_none() {
                last_anchor = Some(a);
                println!(
                    "circle(({}, {}), radius: 2pt, fill: green.transparentize(50%))",
                    a.x, a.y
                );
            } else if a.distance(*b) < 0.00001 {
                last_anchor = None;
                subpath_cnt += 1;
                println!(
                    "circle(({}, {}), radius: 4pt, fill: red.transparentize(50%))",
                    a.x, a.y
                );
            } else {
                println!("circle(({}, {}), radius: 2pt, fill: none)", a.x, a.y);
            }
            println!(
                "circle(({}, {}), radius: 1pt, fill: gray, stroke: none)",
                b.x, b.y
            );

            if i == segs.len() - 1 {
                println!(
                    "circle(({}, {}), radius: 4pt, fill: red.transparentize(50%))",
                    c.x, c.y
                );
            }

            if a.distance(*b) > 0.00001 {
                println!(
                    "bezier(({}, {}), ({}, {}), ({}, {}), stroke: {})",
                    a.x, a.y, c.x, c.y, b.x, b.y, colors[subpath_cnt]
                );
            }
        });
    }

    #[test]
    fn test_foo() {
        let svg = SvgItem::new(typst_svg("R")).with(|svg| {
            svg.scale_to_with_stroke(ScaleHint::PorportionalY(4.0))
                .move_to(dvec3(2.0, 2.0, 0.0));
        });
        // println!("{:?}", svg.0[0].vpoints);
        let points = (svg.0[0].vpoints.0).clone();

        print_typst_vitem(points);
    }

    #[test]
    fn test_foo2() {
        let angle = PI / 3.0 * 2.0;
        let arc = Arc::new(angle, 2.0).with(|arc| {
            arc.rotate(PI / 2.0 - angle / 2.0, DVec3::Z)
                .move_to(dvec3(2.0, 2.0, 0.0));
        });
        let arc = VItem::from(arc);
        let points = (*arc.vpoints).clone();
        println!("{points:?}");

        print_typst_vitem(points);
    }
}
