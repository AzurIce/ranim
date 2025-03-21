use std::{cmp::Ordering, f32, path::Path, slice::Iter, vec};

use color::{AlphaColor, Srgb, palette::css};
use glam::{Affine2, Vec3, vec3};
use log::warn;

use crate::{
    color::{rgb8, rgba},
    components::{Anchor, vpoint::VPoint},
    context::WgpuContext,
    prelude::{Alignable, Empty, Fill, Interpolatable, Opacity, Partial, Stroke, Transformable},
    render::primitives::{
        ExtractFrom, RenderInstance, RenderInstances, svg_item::SvgItemPrimitive,
    },
    utils::{bezier::PathBuilder, math::interpolate_usize},
};

use super::{Entity, group::Group, vitem::VItem};

#[derive(Debug, Clone)]
pub struct SvgItem {
    pub vitems: Vec<VItem>,
}

impl From<VItem> for SvgItem {
    fn from(value: VItem) -> Self {
        Self {
            vitems: vec![value],
        }
    }
}

// MARK: Transformable

impl Transformable<VPoint> for SvgItem {
    fn apply_points_function(
        &mut self,
        f: impl Fn(&mut crate::components::ComponentVec<VPoint>) + Copy,
        anchor: Anchor,
    ) -> &mut Self {
        let point = match anchor {
            Anchor::Edge(edge) => self.get_bounding_box_point(edge),
            Anchor::Point(point) => point,
        };
        // println!("{:?}, {:?}", anchor, point);
        self.vitems.iter_mut().for_each(|x| {
            x.apply_points_function(f, Anchor::Point(point));
        });
        self
    }
    fn get_bounding_box(&self) -> [Vec3; 3] {
        let [min, max] = self
            .vitems
            .iter()
            .map(|x| x.get_bounding_box())
            .map(|[min, _, max]| [min, max])
            .reduce(|[acc_min, acc_max], [min, max]| [acc_min.min(min), acc_max.max(max)])
            .unwrap();
        [min, (min + max) / 2., max]
    }
    fn get_start_position(&self) -> Option<Vec3> {
        self.vitems
            .first()
            .and_then(|vitem| vitem.get_start_position())
    }
    fn get_end_position(&self) -> Option<Vec3> {
        self.vitems
            .first()
            .and_then(|vitem| vitem.get_start_position())
    }
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

        let vitems = vitems_from_tree(&tree);
        let mut svg_item = Self { vitems };
        svg_item.put_center_on(Vec3::ZERO);
        svg_item.rotate(f32::consts::PI, Vec3::X);
        svg_item
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
    fn get_render_instance_for_entity<'a>(
        &self,
        render_instances: &'a RenderInstances,
        entity_id: usize,
    ) -> Option<&'a dyn RenderInstance> {
        render_instances
            .get_dynamic::<SvgItemPrimitive>(entity_id)
            .map(|x| x as &dyn RenderInstance)
    }
    fn prepare_render_instance_for_entity(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        entity_id: usize,
    ) {
        let render_instance = render_instances.get_dynamic_or_init::<SvgItemPrimitive>(entity_id);
        render_instance.update_from(ctx, self);
    }
}

// MARK: Extract impl

impl ExtractFrom<SvgItem> for SvgItemPrimitive {
    fn update_from(&mut self, ctx: &crate::context::WgpuContext, data: &SvgItem) {
        // trace!("SvgItemPrimitive update vitems: {}", data.vitems.len());
        if data.vitems.len() != self.vitem_primitives.len() {
            // trace!("resizing vitem_primitives from {} to {}...", self.vitem_primitives.len(), data.vitems.len());
            self.vitem_primitives
                .resize_with(data.vitems.len(), Default::default);
        }
        // trace!("updating vitem_primitives...");
        self.vitem_primitives
            .iter_mut()
            .zip(data.vitems.iter())
            .for_each(|(vitem_primitive, vitem)| {
                ExtractFrom::<VItem>::update_from(vitem_primitive, ctx, vitem)
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
impl Stroke for SvgItem {
    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.vitems.iter_mut().for_each(|vitem| {
            vitem.set_stroke_color(color);
        });
        self
    }
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.vitems.iter_mut().for_each(|vitem| {
            vitem.set_stroke_opacity(opacity);
        });
        self
    }
    fn set_stroke_width(&mut self, width: f32) -> &mut Self {
        self.vitems.iter_mut().for_each(|vitem| {
            vitem.set_stroke_width(width);
        });
        self
    }
}

impl Fill for SvgItem {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.vitems
            .first()
            .map(|vitem| vitem.fill_color())
            .unwrap_or(css::WHITE)
    }
    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.vitems.iter_mut().for_each(|vitem| {
            vitem.set_fill_color(color);
        });
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.vitems.iter_mut().for_each(|vitem| {
            vitem.set_fill_opacity(opacity);
        });
        self
    }
}

impl Opacity for SvgItem {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.vitems.iter_mut().for_each(|vitem| {
            vitem.set_opacity(opacity);
        });
        self
    }
}

impl Alignable for SvgItem {
    fn is_aligned(&self, other: &Self) -> bool {
        self.vitems.len() == other.vitems.len()
            && self
                .vitems
                .iter()
                .zip(other.vitems.iter())
                .all(|(a, b)| a.is_aligned(b))
    }
    fn align_with(&mut self, other: &mut Self) {
        match self.vitems.len().cmp(&other.vitems.len()) {
            Ordering::Less => {
                self.vitems.resize_with(other.vitems.len(), VItem::empty);
            }
            Ordering::Greater => {
                other.vitems.resize_with(self.vitems.len(), VItem::empty);
            }
            _ => (),
        }
        self.vitems
            .iter_mut()
            .zip(other.vitems.iter_mut())
            .for_each(|(a, b)| a.align_with(b));
    }
}

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
        let max_vitem_idx = self.vitems.len();
        let (start_index, start_residue) = interpolate_usize(0, max_vitem_idx, range.start);
        let (end_index, end_residue) = interpolate_usize(0, max_vitem_idx, range.end);
        // trace!("range: {:?}, start: {} {}, end: {} {}", range, start_index, start_residue, end_index, end_residue);
        let vitems = if start_index == end_index {
            let start_v = self
                .vitems
                .get(start_index)
                .unwrap()
                .get_partial(start_residue..end_residue);
            // .lerp(self.vitems.get(start_index + 1).unwrap(), start_residue);
            vec![start_v]
        } else {
            let mut partial = Vec::with_capacity(end_index - start_index + 1 + 2);
            let start_v = self
                .vitems
                .get(start_index)
                .unwrap()
                .get_partial(start_residue..1.0);
            partial.push(start_v);

            if start_index < end_index - 1 {
                let mid = self.vitems.get(start_index + 1..end_index).unwrap();
                partial.extend_from_slice(mid);
            }

            if end_residue != 0.0 {
                // .lerp(self.vitems.get(start_index + 1).unwrap(), start_residue);
                let end_v = self
                    .vitems
                    .get(end_index)
                    .unwrap()
                    .get_partial(0.0..end_residue);
                // .lerp(self.vitems.get(end_index + 1).unwrap(), end_residue);

                partial.push(end_v);
            }
            partial
        };

        Self { vitems }
    }
}

// MARK: Another aproach

impl Group<VItem> {
    pub fn from_svg(svg: impl AsRef<str>) -> Self {
        let svg = svg.as_ref();
        let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();

        let mut vitem_group = Self(vitems_from_tree(&tree));
        vitem_group.put_center_on(Vec3::ZERO);
        vitem_group.rotate(f32::consts::PI, Vec3::X);
        vitem_group
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

fn vitems_from_tree(tree: &usvg::Tree) -> Vec<VItem> {
    let mut vitems = vec![];
    for (path, transform) in walk_svg_group(tree.root()) {
        // let transform = path.abs_transform();

        let mut builder = PathBuilder::new();
        for segment in path.data().segments() {
            match segment {
                usvg::tiny_skia_path::PathSegment::MoveTo(p) => {
                    // println!("MoveTo: {:?}", vec3(p.x, p.y, 0.0));
                    builder.move_to(vec3(p.x, p.y, 0.0))
                }
                usvg::tiny_skia_path::PathSegment::LineTo(p) => {
                    // println!("LineTo: {:?}", vec3(p.x, p.y, 0.0));
                    builder.line_to(vec3(p.x, p.y, 0.0))
                }
                usvg::tiny_skia_path::PathSegment::QuadTo(p1, p2) => {
                    // println!("QuadTo: {:?}, {:?}", vec3(p1.x, p1.y, 0.0), vec3(p2.x, p2.y, 0.0));
                    builder.quad_to(vec3(p1.x, p1.y, 0.0), vec3(p2.x, p2.y, 0.0))
                }
                usvg::tiny_skia_path::PathSegment::CubicTo(p1, p2, p3) => {
                    // println!("CubicTo: {:?}, {:?}, {:?}", vec3(p1.x, p1.y, 0.0), vec3(p2.x, p2.y, 0.0), vec3(p3.x, p3.y, 0.0));
                    builder.cubic_to(
                        vec3(p1.x, p1.y, 0.0),
                        vec3(p2.x, p2.y, 0.0),
                        vec3(p3.x, p3.y, 0.0),
                    )
                }
                usvg::tiny_skia_path::PathSegment::Close => {
                    // println!("Close");
                    builder.close_path()
                }
            };
        }
        if builder.is_empty() {
            warn!("empty path");
            continue;
        }

        let mut vitem = VItem::from_vpoints(builder.vpoints().to_vec());
        let affine = Affine2::from_cols_array(&[
            transform.sx,
            transform.kx,
            transform.kx,
            transform.sy,
            transform.tx,
            transform.ty,
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
    use crate::typst_svg;

    const SVG: &str = include_str!("../../assets/Ghostscript_Tiger.svg");
    #[test]
    fn test_svg_element_iter() {
        let tree = usvg::Tree::from_str(SVG, &usvg::Options::default()).unwrap();
        let paths = walk_svg_group(tree.root()).collect::<Vec<_>>();
        println!("{} paths", paths.len());
    }

    #[test]
    fn test_get_partial() {
        let mut svg = SvgItem::from_svg(typst_svg!("R"));
        svg.scale(Vec3::splat(10.0));

        println!("{:?}", svg.vitems[0].vpoints);
        let partial = svg.get_partial(0.0..0.5);
        println!("{:?}", partial.vitems[0].vpoints);
        let partial = svg.vitems[0].get_partial(0.0..0.5);
        println!("{:?}", partial.vpoints);
    }
}
