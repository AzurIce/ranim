use std::collections::BTreeMap;

use ranim_core::{
    color::{AlphaColor, Srgb, rgba},
    glam::{DAffine2, DVec2, DVec3, Vec3Swizzles},
    utils::bezier::PathBuilder,
};
use ttf_parser::{GlyphId, OutlineBuilder};
use typst::{
    layout::{Abs, Frame, FrameItem, GroupItem, Point, Ratio, Transform},
    text::{TextItem, color::should_outline},
    visualize::{CurveItem, FillRule, Geometry, Paint, Shape},
};
use typst_layout::{Page, PagedDocument};

use crate::{
    CompileOptions, GlyphInfo, TypstDocument, TypstPage, TypstPath, TypstStroke, TypstWarning,
};

pub(crate) fn collect(
    document: &PagedDocument,
    options: CompileOptions,
    warnings: &mut Vec<TypstWarning>,
) -> TypstDocument {
    let pages = document
        .pages()
        .iter()
        .map(|page| PageCollector::new(options, warnings).collect_page(page))
        .collect();
    TypstDocument { pages }
}

struct PageCollector<'a> {
    options: CompileOptions,
    paths: Vec<TypstPath>,
    groups: BTreeMap<String, Vec<usize>>,
    glyphs: Vec<GlyphInfo>,
    active_labels: Vec<String>,
    warnings: &'a mut Vec<TypstWarning>,
}

impl<'a> PageCollector<'a> {
    fn new(options: CompileOptions, warnings: &'a mut Vec<TypstWarning>) -> Self {
        Self {
            options,
            paths: Vec::new(),
            groups: BTreeMap::new(),
            glyphs: Vec::new(),
            active_labels: Vec::new(),
            warnings,
        }
    }

    fn collect_page(mut self, page: &Page) -> TypstPage {
        if self.options.include_page_fill
            && let Some(fill) = page.fill_or_white()
        {
            let shape = Geometry::Rect(page.frame.size() + page.bleed.sum_by_axis()).filled(fill);
            let transform = Transform::translate(-page.bleed.left, -page.bleed.top);
            self.collect_shape(transform, &shape);
        }
        self.collect_frame(Transform::identity(), &page.frame);

        for path in &mut self.paths {
            for point in &mut path.points {
                point.y = -point.y;
            }
        }
        if self.options.center_content {
            center_paths(&mut self.paths);
        }

        TypstPage {
            size: DVec2::new(page.frame.width().to_pt(), page.frame.height().to_pt()),
            paths: self.paths,
            groups: self.groups,
            glyphs: self.glyphs,
        }
    }

    fn collect_frame(&mut self, transform: Transform, frame: &Frame) {
        for (pos, item) in frame.items() {
            let item_transform = transform.pre_concat(Transform::translate(pos.x, pos.y));
            match item {
                FrameItem::Group(group) => self.collect_group(item_transform, group),
                FrameItem::Text(text) => self.collect_text(item_transform, text),
                FrameItem::Shape(shape, _) => self.collect_shape(item_transform, shape),
                FrameItem::Image(_, _, _) => self.warn(TypstWarning::ImageUnsupported),
                FrameItem::Link(_, _) | FrameItem::Tag(_) => {}
            }
        }
    }

    fn collect_group(&mut self, transform: Transform, group: &GroupItem) {
        let transform = transform.pre_concat(group.transform);
        if group.clip.is_some() {
            self.warn(TypstWarning::ClipPathUnsupported);
        }
        if let Some(label) = group.label {
            self.active_labels.push(label.resolve().to_string());
            self.collect_frame(transform, &group.frame);
            self.active_labels.pop();
        } else {
            self.collect_frame(transform, &group.frame);
        }
    }

    fn collect_text(&mut self, transform: Transform, text: &TextItem) {
        let transform = transform.pre_concat(Transform::scale(Ratio::one(), -Ratio::one()));
        let mut x = Abs::zero();
        let mut y = Abs::zero();

        for glyph in &text.glyphs {
            let glyph_id = GlyphId(glyph.id);
            let x_offset = x + glyph.x_offset.at(text.size);
            let y_offset = y + glyph.y_offset.at(text.size);
            let glyph_transform = transform.pre_concat(Transform::translate(x_offset, y_offset));
            let item_index = self.collect_glyph(glyph_transform, text, glyph_id);
            let text_range = glyph.range();
            self.glyphs.push(GlyphInfo {
                item_index,
                text: text
                    .text
                    .get(text_range.clone())
                    .unwrap_or_default()
                    .to_owned(),
                text_range,
            });
            x += glyph.x_advance.at(text.size);
            y += glyph.y_advance.at(text.size);
        }
    }

    fn collect_glyph(
        &mut self,
        transform: Transform,
        text: &TextItem,
        glyph_id: GlyphId,
    ) -> Option<usize> {
        if !should_outline(&text.font, glyph_id) {
            self.warn(TypstWarning::ColorGlyphUnsupported);
            return None;
        }

        let scale = text.size.to_pt() / text.font.units_per_em();
        let mut builder = GlyphPathBuilder::new(scale);
        text.font.ttf().outline_glyph(glyph_id, &mut builder)?;
        let path = builder.finish();
        if path.is_empty() {
            return None;
        }

        let fill = Some(self.paint(&text.fill));
        let stroke = text.stroke.as_ref().map(|stroke| TypstStroke {
            color: self.paint(&stroke.paint),
            width: (stroke.thickness.to_pt() * transform_scale(transform)) as f32,
        });
        let points = transform_points(path.vpoints(), transform);
        Some(self.push_path(TypstPath {
            points,
            fill,
            stroke,
        }))
    }

    fn collect_shape(&mut self, transform: Transform, shape: &Shape) {
        let mut path = PathBuilder::new();
        match &shape.geometry {
            Geometry::Line(end) => {
                path.move_to(DVec3::ZERO).line_to(point(*end));
            }
            Geometry::Rect(size) => {
                path.move_to(DVec3::ZERO)
                    .line_to(DVec3::new(0.0, size.y.to_pt(), 0.0))
                    .line_to(DVec3::new(size.x.to_pt(), size.y.to_pt(), 0.0))
                    .line_to(DVec3::new(size.x.to_pt(), 0.0, 0.0))
                    .close_path();
            }
            Geometry::Curve(curve) => {
                for item in &curve.0 {
                    match *item {
                        CurveItem::Move(pos) => {
                            path.move_to(point(pos));
                        }
                        CurveItem::Line(pos) => {
                            path.line_to(point(pos));
                        }
                        CurveItem::Cubic(a, b, end) => {
                            path.cubic_to(point(a), point(b), point(end));
                        }
                        CurveItem::Close => {
                            path.close_path();
                        }
                    }
                }
            }
        }
        if path.is_empty() {
            return;
        }

        if shape.fill_rule == FillRule::EvenOdd {
            self.warn(TypstWarning::EvenOddFillUnsupported);
        }
        let fill = shape.fill.as_ref().map(|paint| self.paint(paint));
        let stroke = shape.stroke.as_ref().map(|stroke| TypstStroke {
            color: self.paint(&stroke.paint),
            width: (stroke.thickness.to_pt() * transform_scale(transform)) as f32,
        });
        let points = transform_points(path.vpoints(), transform);
        self.push_path(TypstPath {
            points,
            fill,
            stroke,
        });
    }

    fn paint(&mut self, paint: &Paint) -> AlphaColor<Srgb> {
        match paint {
            Paint::Solid(color) => {
                let (r, g, b, a) = color.to_rgb().into_components();
                rgba(r, g, b, a)
            }
            Paint::Gradient(_) => {
                self.warn(TypstWarning::GradientUnsupported);
                opaque_white()
            }
            Paint::Tiling(_) => {
                self.warn(TypstWarning::TilingUnsupported);
                opaque_white()
            }
        }
    }

    fn push_path(&mut self, path: TypstPath) -> usize {
        let index = self.paths.len();
        self.paths.push(path);
        for label in &self.active_labels {
            self.groups.entry(label.clone()).or_default().push(index);
        }
        index
    }

    fn warn(&mut self, warning: TypstWarning) {
        if !self.warnings.contains(&warning) {
            self.warnings.push(warning);
        }
    }
}

fn point(point: Point) -> DVec3 {
    DVec3::new(point.x.to_pt(), point.y.to_pt(), 0.0)
}

fn transform_points(points: &[DVec3], transform: Transform) -> Vec<DVec3> {
    let affine = DAffine2::from_cols_array(&[
        transform.sx.get(),
        transform.ky.get(),
        transform.kx.get(),
        transform.sy.get(),
        transform.tx.to_pt(),
        transform.ty.to_pt(),
    ]);
    points
        .iter()
        .map(|point| affine.transform_point2(point.xy()).extend(point.z))
        .collect()
}

fn transform_scale(transform: Transform) -> f64 {
    let x = transform.sx.get().hypot(transform.ky.get());
    let y = transform.kx.get().hypot(transform.sy.get());
    ((x * x + y * y) / 2.0).sqrt()
}

fn center_paths(paths: &mut [TypstPath]) {
    let mut min = DVec3::splat(f64::INFINITY);
    let mut max = DVec3::splat(f64::NEG_INFINITY);
    for point in paths.iter().flat_map(|path| &path.points) {
        min = min.min(*point);
        max = max.max(*point);
    }
    if min.is_finite() && max.is_finite() {
        let center = (min + max) / 2.0;
        for point in paths.iter_mut().flat_map(|path| &mut path.points) {
            *point -= center;
        }
    }
}

fn opaque_white() -> AlphaColor<Srgb> {
    rgba(1.0, 1.0, 1.0, 1.0)
}

struct GlyphPathBuilder {
    scale: f64,
    path: PathBuilder,
}

impl GlyphPathBuilder {
    fn new(scale: f64) -> Self {
        Self {
            scale,
            path: PathBuilder::new(),
        }
    }

    fn finish(self) -> PathBuilder {
        self.path
    }

    fn point(&self, x: f32, y: f32) -> DVec3 {
        DVec3::new(x as f64 * self.scale, y as f64 * self.scale, 0.0)
    }
}

impl OutlineBuilder for GlyphPathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to(self.point(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to(self.point(x, y));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.path.quad_to(self.point(x1, y1), self.point(x, y));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.path
            .cubic_to(self.point(x1, y1), self.point(x2, y2), self.point(x, y));
    }

    fn close(&mut self) {
        self.path.close_path();
    }
}
