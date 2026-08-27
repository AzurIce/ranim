use color::{AlphaColor, Srgb, palette::css, rgb8, rgba};
use glam::{DAffine2, DAffine3, DMat3, DVec3, dvec3};
use ranim_core::anchor::Aabb;
use ranim_core::core_item::CoreItem;
use ranim_core::traits::{ApplyTransform, FillColor, Opacity, StrokeColor, StrokeWidth};
use ranim_core::traits::{PointsFunc, Rigid, TransformGroup};
use ranim_core::utils::bezier::PathBuilder;
use ranim_core::{Extract, color, components::width::Width, glam};
use tracing::warn;

use super::VItem;
use crate::hierarchy::Node;

// MARK: ### SvgItem ###
/// An Svg Item.
///
/// Its inner is a [`Node`] tree of [`VItem`]s that mirrors the group
/// structure of the source SVG: every `usvg` group becomes a group node
/// carrying its **relative** transform (never baked into point data), and
/// every path becomes a leaf holding its raw geometry. The root node's
/// transform carries the whole placement, so repositioning an [`SvgItem`]
/// is O(1) and never rewrites points.
///
/// Use [`Vec::<VItem>::from`] to flatten into baked items, and
/// [`SvgItem::by_id`] to address subtrees by their SVG element id.
#[derive(Clone, Debug)]
pub struct SvgItem(Node<VItem>);

impl From<SvgItem> for Vec<VItem> {
    /// Flatten the hierarchy depth-first (painter's-algorithm order),
    /// baking each leaf's accumulated world affine into a clone of its
    /// [`VItem`].
    fn from(value: SvgItem) -> Self {
        value
            .0
            .leaves()
            .map(|(world, leaf)| {
                let mut vitem = leaf.clone();
                vitem.apply_affine3(world);
                vitem
            })
            .collect()
    }
}

impl SvgItem {
    /// Creates a new SvgItem from a SVG string
    ///
    /// The tree is centered on its own bounding box and flipped over the
    /// x axis (SVG's y-down coordinates become y-up), matching the
    /// composition of the old `move_to(ZERO)` + `rotate_on_x(PI)` pipeline
    /// — but as an O(1) root pose instead of baked points.
    pub fn new(svg: impl AsRef<str>) -> Self {
        let tree =
            usvg::Tree::from_str(svg.as_ref(), &usvg::Options::default()).expect("invalid svg");
        let mut item = Self::from_tree(&tree);
        let [min, max] = item.0.aabb();
        let center = (min + max) * 0.5;
        item.0.transform = DAffine3::from(
            Rigid::from_axis_angle(DVec3::X, std::f64::consts::PI)
                .compose(&Rigid::from_translation(-center)),
        );
        item
    }

    /// Build the hierarchy from a `usvg` tree, preserving its structure.
    ///
    /// Unlike [`SvgItem::new`], no root normalization happens: coordinates
    /// stay in the SVG's local (y-down) space. This keeps
    /// [`vitems_from_tree`] behaving exactly like its pre-hierarchy
    /// counterpart.
    pub fn from_tree(tree: &usvg::Tree) -> Self {
        Self(build_group_node(tree.root(), usvg::Transform::identity()))
    }

    /// The underlying hierarchy tree.
    pub fn tree(&self) -> &Node<VItem> {
        &self.0
    }

    /// The underlying hierarchy tree, mutably.
    pub fn tree_mut(&mut self) -> &mut Node<VItem> {
        &mut self.0
    }

    /// Consume the item, returning the underlying hierarchy tree.
    pub fn into_tree(self) -> Node<VItem> {
        self.0
    }

    /// The leaf payload of the first node (depth-first) whose id matches,
    /// or — when the id sits on a group — the first leaf of that subtree.
    ///
    /// SVG ids may be set on groups, so such lookups resolve to the group's
    /// first leaf; use [`SvgItem::tree`] for structural access.
    pub fn by_id(&self, id: &str) -> Option<&VItem> {
        let node = self.0.by_id(id)?;
        node.item().or_else(|| node.first_leaf())
    }

    /// Mutable variant of [`SvgItem::by_id`]. Mutation reaches only the
    /// matched leaf's local (canonical) data; node transforms are
    /// unaffected.
    pub fn by_id_mut(&mut self, id: &str) -> Option<&mut VItem> {
        let node = self.0.by_id_mut(id)?;
        if node.item_mut().is_some() {
            node.item_mut()
        } else {
            node.first_leaf_mut()
        }
    }
}

// MARK: Trait impls
impl Aabb for SvgItem {
    fn aabb(&self) -> [glam::DVec3; 2] {
        self.0.aabb()
    }
}

impl<G: Into<glam::DAffine3>> ApplyTransform<G> for SvgItem {
    fn apply(&mut self, transform: G) -> &mut Self {
        self.0.apply(transform.into());
        self
    }
}

impl FillColor for SvgItem {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.0.fill_color()
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
        self.0.stroke_color()
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
        self.0.set_opacity(opacity);
        self
    }
}

impl StrokeWidth for SvgItem {
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

/// Build a group node from a `usvg` group and the absolute transform of
/// its parent group.
///
/// The node's transform is the group's transform **relative** to its
/// parent (`parent_abs^-1 * group_abs`), so path data is never rewritten
/// and re-posing any level of the tree is a local operation.
fn build_group_node(group: &usvg::Group, parent_abs: usvg::Transform) -> Node<VItem> {
    let abs = group.abs_transform();
    let children: Vec<Node<VItem>> = group
        .children()
        .iter()
        .filter_map(|node| match node {
            usvg::Node::Group(group) => Some(build_group_node(group, abs)),
            usvg::Node::Path(path) => build_path_leaf(path, abs),
            // Image and Text nodes are not supported and are skipped.
            usvg::Node::Image(_) | usvg::Node::Text(_) => None,
        })
        .collect();
    let mut node = Node::group(children);
    node.transform = widen_to_daffine3(rel_transform(parent_abs, abs));
    if !group.id().is_empty() {
        node.id = Some(group.id().to_string());
    }
    node
}

/// Build a leaf node from a `usvg` path and the absolute transform of its
/// parent group.
///
/// The [`VItem`] is built from the raw path segments exactly like the old
/// flat walker did (same segment mapping, fill/stroke parsing, and empty
/// path skipping) — but the path's placement goes onto the leaf node's
/// transform instead of being baked into the points.
fn build_path_leaf(path: &usvg::Path, parent_abs: usvg::Transform) -> Option<Node<VItem>> {
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
        return None;
    }

    let mut vitem = VItem::from_vpoints(builder.vpoints().to_vec());
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

    let mut node = Node::leaf(vitem);
    node.transform = widen_to_daffine3(rel_transform(parent_abs, path.abs_transform()));
    if !path.id().is_empty() {
        node.id = Some(path.id().to_string());
    }
    Some(node)
}

/// The transform of a node relative to its parent: `parent_abs^-1 * abs`.
///
/// A singular (non-invertible) parent — e.g. `scale(0)` — has no relative
/// form; we warn and fall back to the identity, leaving the child's data
/// in its raw coordinates.
fn rel_transform(parent_abs: usvg::Transform, abs: usvg::Transform) -> DAffine2 {
    let parent = usvg_transform_to_daffine2(parent_abs);
    if parent.matrix2.determinant() == 0.0 {
        warn!("singular svg transform cannot be inverted, using identity");
        return DAffine2::IDENTITY;
    }
    parent.inverse() * usvg_transform_to_daffine2(abs)
}

/// Convert a `usvg` (tiny-skia) row-major transform into a [`DAffine2`].
fn usvg_transform_to_daffine2(transform: usvg::Transform) -> DAffine2 {
    DAffine2::from_cols_array(&[
        transform.sx as f64,
        transform.ky as f64,
        transform.kx as f64,
        transform.sy as f64,
        transform.tx as f64,
        transform.ty as f64,
    ])
}

/// Widen a 2D affine into a 3D one, embedding the xy plane at z = 0.
fn widen_to_daffine3(affine: DAffine2) -> DAffine3 {
    DAffine3::from_mat3_translation(
        DMat3::from_cols(
            affine.matrix2.x_axis.extend(0.0),
            affine.matrix2.y_axis.extend(0.0),
            DVec3::Z,
        ),
        affine.translation.extend(0.0),
    )
}

/// The first leaf (depth-first) under the first node whose id matches.
/// Construct a `Vec<VItem` from `&str` of a SVG
pub fn vitems_from_svg(svg: &str) -> Vec<VItem> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    vitems_from_tree(&tree)
}

/// Construct a `Vec<VItem>` from `&usvg::Tree`
pub fn vitems_from_tree(tree: &usvg::Tree) -> Vec<VItem> {
    Vec::<VItem>::from(SvgItem::from_tree(tree))
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use glam::dvec2;
    use ranim_core::traits::ShiftTransform;

    use super::*;

    /// A source SVG with nested transformed groups and ids on groups and
    /// paths. `width`/`height` match the viewBox, so usvg adds no root
    /// scaling and the root group's absolute transform is the identity.
    const NESTED_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="400" viewBox="0 0 400 400">
        <g transform="translate(10) scale(2)">
            <g id="inner" transform="rotate(45)">
                <path id="leaf-a" d="M 10 10 L 20 10" fill="#ff0000" stroke="#00ff00" stroke-width="0.5"/>
            </g>
            <path id="leaf-b" d="M 0 0 L 5 0" fill="#0000ff"/>
        </g>
    </svg>"##;

    fn parse(svg: &str) -> usvg::Tree {
        usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap()
    }

    /// `translate(10) scale(2)`: the scale applies to the points first.
    fn outer_rel() -> DAffine2 {
        DAffine2::from_translation(dvec2(10.0, 0.0)) * DAffine2::from_scale(dvec2(2.0, 2.0))
    }

    /// Affine equality with a tolerance suited to the f32 transforms usvg
    /// stores.
    fn assert_affine3_eq(actual: DAffine3, expected: DAffine3) {
        assert!(
            actual.translation.abs_diff_eq(expected.translation, 1e-6),
            "translation {:?} vs {:?}",
            actual.translation,
            expected.translation
        );
        for i in 0..3 {
            assert!(
                actual
                    .matrix3
                    .col(i)
                    .abs_diff_eq(expected.matrix3.col(i), 1e-6),
                "matrix3 column {i} diverges"
            );
        }
    }

    #[test]
    fn structure_ids_and_relative_transforms_match_the_source() {
        let svg = SvgItem::from_tree(&parse(NESTED_SVG));
        let root = svg.tree();

        assert!(root.is_group());
        assert_eq!(root.id, None);
        // No viewBox scaling: the root group's relative transform is the
        // identity before normalization.
        assert_eq!(root.transform, DAffine3::IDENTITY);

        let children = root.children();
        assert_eq!(children.len(), 1);
        let outer = &children[0];
        assert!(outer.is_group());
        assert_eq!(outer.id, None, "the outer <g> has no id attribute");
        assert_affine3_eq(outer.transform, widen_to_daffine3(outer_rel()));

        let inner = &outer.children()[0];
        assert_eq!(inner.id.as_deref(), Some("inner"));
        assert!(inner.is_group());
        assert_affine3_eq(
            inner.transform,
            widen_to_daffine3(DAffine2::from_angle(45.0f64.to_radians())),
        );

        let leaf_a = &inner.children()[0];
        assert_eq!(leaf_a.id.as_deref(), Some("leaf-a"));
        assert!(leaf_a.is_leaf());
        assert_eq!(leaf_a.transform, DAffine3::IDENTITY);
        // Path data stays in raw SVG coordinates; the placement lives on
        // the node transforms. (The builder's middle point is the line's
        // midpoint handle.)
        let item = leaf_a.item().unwrap();
        assert_eq!(item.vpoints.0[0], dvec3(10.0, 10.0, 0.0));
        assert_eq!(item.vpoints.0[2], dvec3(20.0, 10.0, 0.0));
    }

    #[test]
    fn flattened_world_points_match_hand_composed_affine() {
        let svg = SvgItem::from_tree(&parse(NESTED_SVG));
        let vitems = Vec::<VItem>::from(svg.clone());

        // leaf-a world = root(identity) * outer * inner * leaf(identity).
        // usvg transforms are f32, so world values carry ~1e-6 noise.
        let expected_a = widen_to_daffine3(outer_rel() * DAffine2::from_angle(PI / 4.0));
        let raw_a = svg.by_id("leaf-a").unwrap().vpoints.0.clone();
        for (world, raw) in vitems[0].vpoints.0.iter().zip(raw_a.iter()) {
            assert!(
                world.abs_diff_eq(expected_a.transform_point3(*raw), 1e-5),
                "world {world:?} vs expected {:?}",
                expected_a.transform_point3(*raw)
            );
        }

        // leaf-b only sees the outer group's transform.
        let expected_b = widen_to_daffine3(outer_rel());
        let raw_b = svg.by_id("leaf-b").unwrap().vpoints.0.clone();
        for (world, raw) in vitems[1].vpoints.0.iter().zip(raw_b.iter()) {
            assert!(
                world.abs_diff_eq(expected_b.transform_point3(*raw), 1e-5),
                "world {world:?} vs expected {:?}",
                expected_b.transform_point3(*raw)
            );
        }
    }

    #[test]
    fn dfs_leaf_order_matches_source_order() {
        // leaf-a lives inside the inner group, leaf-b is its later
        // sibling's child — depth-first emission must paint leaf-a first.
        let vitems = Vec::<VItem>::from(SvgItem::from_tree(&parse(NESTED_SVG)));
        assert_eq!(vitems.len(), 2);

        // leaf-a's first anchor, hand-composed: (10, 10) rotated by 45deg,
        // scaled by 2, translated by (10, 0). usvg stores f32 transforms.
        assert!(
            vitems[0].vpoints.0[0]
                .abs_diff_eq(dvec3(10.0, 20.0 * std::f64::consts::SQRT_2, 0.0), 1e-5)
        );
        // leaf-b's first anchor is the origin of the outer group.
        assert!(vitems[1].vpoints.0[0].abs_diff_eq(dvec3(10.0, 0.0, 0.0), 1e-5));
    }

    #[test]
    fn new_centers_the_aabb_and_flips_after_centering() {
        // The unnormalized world, to derive the old pipeline's center.
        let raw_items = Vec::<VItem>::from(SvgItem::from_tree(&parse(NESTED_SVG)));
        let [min, max] = raw_items.aabb();
        let center = (min + max) * 0.5;

        let new_items = Vec::<VItem>::from(SvgItem::new(NESTED_SVG));

        // old: move_to(ZERO) then rotate_on_x(PI) — i.e. flip(p - center).
        let flip = DVec3::new(1.0, -1.0, -1.0);
        for (new, raw) in new_items.iter().zip(raw_items.iter()) {
            for (new_p, raw_p) in new.vpoints.0.iter().zip(raw.vpoints.0.iter()) {
                let expected = (raw_p - center) * flip;
                assert!(
                    new_p.abs_diff_eq(expected, 1e-9),
                    "new {new_p:?} vs expected {expected:?}"
                );
            }
        }

        // The extracted aabb is centered at the origin.
        let [min, max] = new_items.aabb();
        assert!(((min + max) * 0.5).abs_diff_eq(DVec3::ZERO, 1e-9));
    }

    #[test]
    fn by_id_lookups_read_and_write_only_the_named_leaf() {
        let mut svg = SvgItem::from_tree(&parse(NESTED_SVG));
        assert!(svg.by_id("missing").is_none());

        let sibling_before = svg.by_id("leaf-b").unwrap().vpoints.0.clone();
        svg.by_id_mut("leaf-a").unwrap().shift(DVec3::X);
        assert!(
            svg.by_id("leaf-a").unwrap().vpoints.0[0].abs_diff_eq(dvec3(11.0, 10.0, 0.0), 1e-9)
        );
        assert_eq!(
            svg.by_id("leaf-b").unwrap().vpoints.0,
            sibling_before,
            "the sibling leaf must be untouched"
        );

        // A group id resolves to its first leaf.
        assert_eq!(
            svg.by_id("inner").unwrap().vpoints.0[0],
            dvec3(11.0, 10.0, 0.0)
        );
    }

    #[test]
    fn ghostscript_tiger_keeps_leaf_count_and_centering() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/Ghostscript_Tiger.svg");
        let source = std::fs::read_to_string(&path).expect("the tiger asset should exist");

        let svg = SvgItem::new(&source);
        let vitems = Vec::<VItem>::from(svg.clone());

        let leaf_count = svg.tree().leaves().count();
        assert!(leaf_count > 100, "unexpectedly few leaves: {leaf_count}");
        assert_eq!(vitems.len(), leaf_count);

        let [min, max] = vitems.aabb();
        assert!(
            ((min + max) * 0.5).abs_diff_eq(DVec3::ZERO, 1e-6),
            "aabb is not centered: min {min:?}, max {max:?}"
        );
    }

    #[test]
    fn style_getters_read_the_first_leaf() {
        let svg = SvgItem::from_tree(&parse(NESTED_SVG));
        // The first leaf is `leaf-a`. Colors round-trip through the VItem's
        // f32 Rgba storage, so compare with a small tolerance.
        let fill = svg.fill_color();
        assert!((fill.components[0] - 1.0).abs() < 1e-6);
        assert!(fill.components[1] < 1e-6 && fill.components[2] < 1e-6);
        let stroke = svg.stroke_color();
        assert!((stroke.components[1] - 1.0).abs() < 1e-6);
        assert!(stroke.components[0] < 1e-6 && stroke.components[2] < 1e-6);
        assert_eq!(svg.stroke_width(), 0.5);
    }
}

#[cfg(all(test, feature = "typst"))]
mod typst_tests {
    use std::f64::consts::PI;

    use glam::dvec3;

    use crate::vitem::{geometry::Arc, typst::typst_svg};
    use ranim_core::{
        anchor::{AabbPoint, Locate},
        traits::{
            RotateTransform, ScaleHint, ScaleTransformExt, ScaleTransformStrokeExt, ShiftTransform,
            ShiftTransformExt, With,
        },
    };

    use super::*;

    #[test]
    fn foo_test_vitems_from_svg() {
        let svg = typst_svg("R");
        let mut vitems = vitems_from_svg(&svg);

        println!("{:?}", vitems.aabb());
        let scale = vitems.calc_scale_ratio(ScaleHint::PorportionalY(8.0));
        println!("scale: {}", scale);
        let center = AabbPoint::CENTER.locate(AsRef::<[VItem]>::as_ref(&vitems));
        println!("{:?}", center);
        vitems
            // .scale_to(ScaleHint::PorportionalY(8.0))
            .move_anchor_to(AabbPoint::CENTER, DVec3::ZERO);

        println!(
            "\n{:?}",
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
        let vitems = Vec::<VItem>::from(svg);
        let points = vitems[0].vpoints.0.clone();

        print_typst_vitem(points);
    }

    #[test]
    fn arc_points_are_transformed_after_conversion_to_vitem() {
        let angle = PI / 3.0 * 2.0;
        let mut arc = VItem::from(Arc::new(angle, 2.0));
        arc.rotate_on_axis(DVec3::Z, PI / 2.0 - angle / 2.0)
            .shift(dvec3(2.0, 2.0, 0.0));
        assert!(arc.vpoints[0].abs_diff_eq(dvec3(3.732050807568877, 3.0, 0.0), 1e-10));
        assert!(
            arc.vpoints[arc.vpoints.len() - 1]
                .abs_diff_eq(dvec3(0.2679491924311228, 3.0, 0.0), 1e-10)
        );
        let points = (*arc.vpoints).clone();
        println!("{points:?}");

        print_typst_vitem(points);
    }
}
