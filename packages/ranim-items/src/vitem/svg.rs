use color::{AlphaColor, Srgb, palette::css, rgb8, rgba};
use glam::DVec3;
use glam::{DAffine2, dvec3};
use ranim_core::anchor::{BoundsAnchor, DBounds3, SemanticBounds};
use ranim_core::core_item::CoreItem;
use ranim_core::traits::{
    PointsFunc, Resize, RotateTransform, ShiftTransformExt, resize_xy_by_bounds,
};
use ranim_core::{Extract, components::width::Width, utils::bezier::PathBuilder};
use ranim_core::{color, glam};
use tracing::warn;

use ranim_core::traits::{FillColor, Opacity, StrokeColor, StrokeWidth};

use super::VItem;

// MARK: ### SvgItem ###
/// An Svg Item
///
/// Its inner is a `Vec<VItem>`
#[derive(
    Clone, ranim_macros::ShiftTransform, ranim_macros::RotateTransform, ranim_macros::Scale,
)]
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
        vitem_group
            .move_to(DVec3::ZERO)
            .rotate_on_x(std::f64::consts::PI);
        vitem_group
    }
}

// MARK: Trait impls
impl SemanticBounds for SvgItem {
    fn semantic_bounds(&self) -> DBounds3 {
        self.0.semantic_bounds()
    }
}

impl Resize<DVec3> for SvgItem {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: DVec3,
    ) -> &mut Self {
        resize_xy_by_bounds(self, bounds, anchor, size.truncate());
        self
    }
}

impl Resize<f64> for SvgItem {
    fn resize_about_bounds(
        &mut self,
        bounds: DBounds3,
        anchor: BoundsAnchor,
        size: f64,
    ) -> &mut Self {
        Resize::<DVec3>::resize_about_bounds(self, bounds, anchor, DVec3::splat(size));
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
        self.0[0].stroke_color()
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
        vitem.apply_affine2(affine);
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
    use super::*;

    #[test]
    fn stroke_color_getter_reads_stroke_not_fill() {
        let mut svg = SvgItem::new(
            r##"<svg viewBox="0 0 10 10" xmlns="http://www.w3.org/2000/svg">
<path d="M1 1 L9 1 L9 9 Z" fill="#ff0000" stroke="#0000ff" stroke-width="1"/>
</svg>"##,
        );
        svg.set_fill_color(rgb8(10, 20, 30));
        svg.set_stroke_color(rgb8(200, 150, 100));

        assert_color_near(svg.fill_color(), rgb8(10, 20, 30));
        assert_color_near(svg.stroke_color(), rgb8(200, 150, 100));
    }

    fn assert_color_near(actual: AlphaColor<Srgb>, expected: AlphaColor<Srgb>) {
        for (actual, expected) in actual.components.into_iter().zip(expected.components) {
            assert!(
                (actual - expected).abs() <= 1.0e-6,
                "{actual} != {expected}"
            );
        }
    }
}
