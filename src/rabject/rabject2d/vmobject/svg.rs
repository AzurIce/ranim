use std::path::Path;

use log::warn;
use usvg::{Options, Tree};

use crate::{
    rabject::{
        rabject2d::bez_path::{BezPath, FillOptions, StrokeOptions},
        Blueprint,
    },
    utils,
};

use super::VMobject;

pub struct Svg(Tree);

impl Svg {
    pub fn from_file(path: impl AsRef<Path>) -> Self {
        let str = std::fs::read_to_string(path).unwrap();
        Self::from_svg(&str)
    }
    pub fn from_svg(svg: &str) -> Self {
        let tree = Tree::from_str(svg, &Options::default()).unwrap();
        Self::from_tree(tree)
    }
    pub fn from_tree(tree: Tree) -> Self {
        Self(tree)
    }
}

impl Blueprint<VMobject> for Svg {
    fn build(self) -> VMobject {
        let subpaths = convert_group_to_subpaths(self.0.root());
        VMobject::new(subpaths)
    }
}

fn convert_group_to_subpaths(group: &usvg::Group) -> Vec<BezPath> {
    let mut subpaths = vec![];
    for node in group.children() {
        let transform = utils::to_affine(&node.abs_transform());
        match node {
            usvg::Node::Path(path) => {
                if let Ok(mut path) = BezPath::try_from(path.as_ref()) {
                    path.apply_affine(transform);
                    subpaths.push(path);
                }
            }
            usvg::Node::Text(_) | usvg::Node::Group(_) => {
                let mut g = convert_group_to_subpaths(match node {
                    usvg::Node::Group(g) => g.as_ref(),
                    usvg::Node::Text(text) => text.flattened(),
                    _ => unreachable!(),
                });
                if matches!(node, usvg::Node::Text(_)) {
                    g.iter_mut().for_each(|p| {
                        p.apply_affine(transform);
                    });
                }
                subpaths.extend(g);
            }
            usvg::Node::Image(img) => match img.kind() {
                usvg::ImageKind::SVG(svg) => {
                    let mut g = convert_group_to_subpaths(svg.root());
                    g.iter_mut().for_each(|p| {
                        p.apply_affine(transform);
                    });
                    subpaths.extend(g);
                }
                _ => {
                    warn!("image is not supported in svg, skipping...");
                    continue;
                }
            },
        }
    }
    subpaths
}

impl TryFrom<&usvg::Path> for BezPath {
    type Error = anyhow::Error;
    fn try_from(path: &usvg::Path) -> Result<Self, Self::Error> {
        if !path.is_visible() {
            anyhow::bail!("path is not visible");
        }
        let inner = utils::to_bez_path(path);

        let fill = path
            .fill()
            .and_then(|f| {
                utils::to_brush(f.paint(), f.opacity()).map(|(brush, transform)| FillOptions {
                    style: match f.rule() {
                        usvg::FillRule::NonZero => vello::peniko::Fill::NonZero,
                        usvg::FillRule::EvenOdd => vello::peniko::Fill::EvenOdd,
                    },
                    brush,
                    transform: Some(transform),
                    opacity: f.opacity().get(),
                })
            })
            .unwrap_or(FillOptions::default().with_opacity(0.0));

        let stroke = path
            .stroke()
            .and_then(|s| {
                utils::to_brush(s.paint(), s.opacity()).map(|(brush, transform)| StrokeOptions {
                    style: utils::to_stroke(s),
                    brush,
                    transform: Some(transform),
                    opacity: s.opacity().get(),
                })
            })
            .unwrap_or(
                StrokeOptions::default()
                    .with_brush(fill.brush.clone())
                    .with_opacity(0.0),
            );

        Ok(BezPath {
            inner,
            stroke,
            fill,
        })
    }
}
