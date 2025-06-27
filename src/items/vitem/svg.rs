use glam::DVec3;

use crate::{
    items::Group,
    render::primitives::{Extract, vitem::VItemPrimitive},
    traits::{BoundingBox, FillColor, Opacity, Rotate, Scale, Shift, StrokeColor, StrokeWidth},
    utils::svg::vitems_from_tree,
};

use super::VItem;

// MARK: ### SvgItem ###
#[derive(Clone)]
pub struct SvgItem(Vec<VItem>);

impl SvgItem {
    pub fn new(svg: impl AsRef<str>) -> Self {
        let svg = svg.as_ref();
        let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();

        let mut vitem_group = Self(vitems_from_tree(&tree));
        vitem_group.put_center_on(DVec3::ZERO);
        vitem_group.rotate(std::f64::consts::PI, DVec3::X);
        vitem_group
    }
}

// MARK: Trait impls
impl BoundingBox for SvgItem {
    fn get_bounding_box(&self) -> [glam::DVec3; 3] {
        self.0.get_bounding_box()
    }
}

impl Shift for SvgItem {
    fn shift(&mut self, shift: glam::DVec3) -> &mut Self {
        self.0.shift(shift);
        self
    }
}

impl Rotate for SvgItem {
    fn rotate_by_anchor(
        &mut self,
        angle: f64,
        axis: glam::DVec3,
        anchor: crate::components::Anchor,
    ) -> &mut Self {
        self.0.rotate_by_anchor(angle, axis, anchor);
        self
    }
}

impl Scale for SvgItem {
    fn scale_by_anchor(
        &mut self,
        scale: glam::DVec3,
        anchor: crate::components::Anchor,
    ) -> &mut Self {
        self.0.scale_by_anchor(scale, anchor);
        self
    }
}

impl FillColor for SvgItem {
    fn fill_color(&self) -> color::AlphaColor<color::Srgb> {
        self.0[0].fill_color()
    }
    fn set_fill_color(&mut self, color: color::AlphaColor<color::Srgb>) -> &mut Self {
        self.0.set_fill_color(color);
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.0.set_fill_opacity(opacity);
        self
    }
}

impl StrokeColor for SvgItem {
    fn stroke_color(&self) -> color::AlphaColor<color::Srgb> {
        self.0[0].fill_color()
    }
    fn set_stroke_color(&mut self, color: color::AlphaColor<color::Srgb>) -> &mut Self {
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
    fn apply_stroke_func(
        &mut self,
        f: impl for<'a> Fn(&'a mut [crate::components::width::Width]),
    ) -> &mut Self {
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
impl From<SvgItem> for Group<VItem> {
    fn from(value: SvgItem) -> Self {
        Self(value.0)
    }
}

impl Extract for SvgItem {
    type Target = Vec<VItemPrimitive>;
    fn extract(&self) -> Self::Target {
        self.0.iter().map(|vitem| vitem.extract()).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use glam::dvec3;

    use crate::{
        items::vitem::{geometry::Arc, typst::typst_svg},
        traits::{ScaleStrokeExt, With},
    };

    use super::*;

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
            svg.scale_to_with_stroke(crate::components::ScaleHint::PorportionalY(4.0))
                .put_center_on(dvec3(2.0, 2.0, 0.0));
        });
        // println!("{:?}", svg.0[0].vpoints);
        let points = svg.0[0].vpoints.0.clone().0;

        print_typst_vitem(points);
    }

    #[test]
    fn test_foo2() {
        let angle = PI / 3.0 * 2.0;
        let arc = Arc::new(angle, 2.0).with(|arc| {
            arc.rotate(PI / 2.0 - angle / 2.0, DVec3::Z)
                .put_center_on(dvec3(2.0, 2.0, 0.0));
        });
        let arc = VItem::from(arc);
        let points = arc.vpoints.0.clone().0;
        println!("{:?}", points);

        print_typst_vitem(points);
    }
}
