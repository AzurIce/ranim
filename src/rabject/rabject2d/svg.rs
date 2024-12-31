use std::fs;

use itertools::Itertools;
use log::trace;
use usvg::{Options, Tree};
use vello::kurbo;

use crate::rabject::rabject2d::bez_path::{FillOptions, StrokeOptions};
use crate::scene::{canvas::camera::CanvasCamera, Entity};
use crate::utils;

use super::bez_path::BezPath;
use super::vmobject::VMobject;

#[derive(Clone)]
pub struct Svg {
    inner: VMobject,
    // tree: usvg::Tree,
    // scene: vello::Scene,
}

impl TryFrom<&usvg::Path> for BezPath {
    type Error = anyhow::Error;
    fn try_from(path: &usvg::Path) -> Result<Self, Self::Error> {
        if !path.is_visible() {
            anyhow::bail!("path is not visible");
        }
        let inner = utils::to_bez_path(path);

        let stroke = path.stroke().and_then(|s| {
            utils::to_brush(s.paint(), s.opacity()).map(|(brush, transform)| StrokeOptions {
                style: utils::to_stroke(s),
                brush,
                transform: Some(transform),
            })
        });

        let fill = path.fill().and_then(|f| {
            utils::to_brush(f.paint(), f.opacity()).map(|(brush, transform)| FillOptions {
                style: match f.rule() {
                    usvg::FillRule::NonZero => vello::peniko::Fill::NonZero,
                    usvg::FillRule::EvenOdd => vello::peniko::Fill::EvenOdd,
                },
                brush,
                transform: Some(transform),
            })
        });

        Ok(BezPath {
            inner,
            stroke,
            fill,
        })
    }
}

impl Into<VMobject> for Svg {
    fn into(self) -> VMobject {
        self.inner
    }
}

impl Svg {
    pub fn from_path(path: &str) -> Self {
        let str = fs::read_to_string(path).unwrap();
        let tree = Tree::from_str(&str, &Options::default()).unwrap();
        Self::from_tree(tree)
    }

    pub fn from_tree(tree: Tree) -> Self {
        // let scene = vello::Scene::new();

        let subpaths = convert_group_to_subpaths(tree.root());
        let vmobject = VMobject::new(subpaths);
        // vmobject.print_tree(2);

        Self {
            inner: vmobject,
            // scene,
            // tree,
        }
    }
}

#[allow(unused)]
impl Entity for Svg {
    type Renderer = CanvasCamera;

    fn tick(&mut self, dt: f32) {}
    fn extract(&mut self) {}
    fn prepare(&mut self, ctx: &crate::context::RanimContext) {}
    fn render(&mut self, ctx: &mut crate::context::RanimContext, renderer: &mut Self::Renderer) {
        self.inner.render(ctx, renderer);
        /* self.scene.reset();
        encode_group_to_scene(
            &mut self.scene,
            &self.tree.root(),
            vello::kurbo::Affine::IDENTITY,
        );
        renderer.vello_scene.append(&self.scene, None); */
    }
}

fn convert_group_to_subpaths(group: &usvg::Group) -> Vec<BezPath> {
    let mut subpaths = vec![];
    for node in group.children() {
        let transform = utils::to_affine(&node.abs_transform());
        match node {
            usvg::Node::Group(g) => {
                // let mut pushed_clip = false;
                // if let Some(clip_path) = g.clip_path() {
                //     trace!("clip path: {}", clip_path.id());
                //     if let Some(usvg::Node::Path(clip_path)) = clip_path.root().children().first() {
                //         // support clip-path with a single path
                //         let local_path = utils::to_bez_path(clip_path);
                //         scene.push_layer(
                //             vello::peniko::BlendMode {
                //                 mix: vello::peniko::Mix::Clip,
                //                 compose: vello::peniko::Compose::SrcOver,
                //             },
                //             1.0,
                //             transform,
                //             &local_path,
                //         );
                //         pushed_clip = true;
                //     }
                // }

                subpaths.extend(convert_group_to_subpaths(g));

                // if pushed_clip {
                //     scene.pop_layer();
                // }
            }
            usvg::Node::Path(path) => {
                if let Ok(mut path) = BezPath::try_from(path.as_ref()) {
                    path.apply_affine(transform);
                    subpaths.push(path);
                }
            }
            _ => unimplemented!(), // usvg::Node::Image(img) => {
                                   //     if !img.is_visible() {
                                   //         continue;
                                   //     }
                                   //     match img.kind() {
                                   //         usvg::ImageKind::JPEG(_)
                                   //         | usvg::ImageKind::PNG(_)
                                   //         | usvg::ImageKind::GIF(_)
                                   //         | usvg::ImageKind::WEBP(_) => {
                                   //             let Ok(decoded_image) = utils::decode_raw_raster_image(img.kind()) else {
                                   //                 // error_handler(scene, node);
                                   //                 continue;
                                   //             };
                                   //             let image = utils::into_image(decoded_image);
                                   //             let image_ts = utils::to_affine(&img.abs_transform());
                                   //             scene.draw_image(&image, image_ts);
                                   //         }
                                   //         usvg::ImageKind::SVG(svg) => {
                                   //             encode_group_to_scene(scene, svg.root(), transform);
                                   //         }
                                   //     }
                                   // }
                                   // usvg::Node::Text(text) => {
                                   //     encode_group_to_scene(scene, text.flattened(), transform);
                                   // }
        }
    }
    subpaths
}

fn encode_group_to_scene(
    scene: &mut vello::Scene,
    group: &usvg::Group,
    transform: vello::kurbo::Affine,
) {
    for node in group.children() {
        let transform = transform * utils::to_affine(&node.abs_transform());
        match node {
            usvg::Node::Group(g) => {
                let mut pushed_clip = false;
                if let Some(clip_path) = g.clip_path() {
                    trace!("clip path: {}", clip_path.id());
                    if let Some(usvg::Node::Path(clip_path)) = clip_path.root().children().first() {
                        // support clip-path with a single path
                        let local_path = utils::to_bez_path(clip_path);
                        scene.push_layer(
                            vello::peniko::BlendMode {
                                mix: vello::peniko::Mix::Clip,
                                compose: vello::peniko::Compose::SrcOver,
                            },
                            1.0,
                            transform,
                            &local_path,
                        );
                        pushed_clip = true;
                    }
                }

                encode_group_to_scene(scene, g, vello::kurbo::Affine::IDENTITY);

                if pushed_clip {
                    scene.pop_layer();
                }
            }
            usvg::Node::Path(path) => {
                if !path.is_visible() {
                    continue;
                }

                let mut local_path = utils::to_bez_path(path);
                local_path.apply_affine(transform);

                let do_fill = |scene: &mut vello::Scene| {
                    if let Some(fill) = &path.fill() {
                        if let Some((brush, brush_transform)) =
                            utils::to_brush(fill.paint(), fill.opacity())
                        {
                            // let brush_transform = brush_transform * transform;
                            scene.fill(
                                match fill.rule() {
                                    usvg::FillRule::NonZero => vello::peniko::Fill::NonZero,
                                    usvg::FillRule::EvenOdd => vello::peniko::Fill::EvenOdd,
                                },
                                kurbo::Affine::IDENTITY,
                                // kurbo::Affine::translate((400.0, 400.0)),
                                // transform,
                                &brush,
                                Some(brush_transform),
                                &local_path,
                            );
                        }
                    }
                };
                let do_stroke = |scene: &mut vello::Scene| {
                    if let Some(stroke) = &path.stroke() {
                        if let Some((brush, brush_transform)) =
                            utils::to_brush(stroke.paint(), stroke.opacity())
                        {
                            let conv_stroke = utils::to_stroke(stroke);
                            scene.stroke(
                                &conv_stroke,
                                kurbo::Affine::IDENTITY,
                                // transform,
                                &brush,
                                Some(brush_transform),
                                &local_path,
                            );
                        }
                    }
                };
                match path.paint_order() {
                    usvg::PaintOrder::FillAndStroke => {
                        do_fill(scene);
                        do_stroke(scene);
                    }
                    usvg::PaintOrder::StrokeAndFill => {
                        do_stroke(scene);
                        do_fill(scene);
                    }
                }
            }
            usvg::Node::Image(img) => {
                if !img.is_visible() {
                    continue;
                }
                match img.kind() {
                    usvg::ImageKind::JPEG(_)
                    | usvg::ImageKind::PNG(_)
                    | usvg::ImageKind::GIF(_)
                    | usvg::ImageKind::WEBP(_) => {
                        let Ok(decoded_image) = utils::decode_raw_raster_image(img.kind()) else {
                            // error_handler(scene, node);
                            continue;
                        };
                        let image = utils::into_image(decoded_image);
                        let image_ts = utils::to_affine(&img.abs_transform());
                        scene.draw_image(&image, image_ts);
                    }
                    usvg::ImageKind::SVG(svg) => {
                        encode_group_to_scene(scene, svg.root(), transform);
                    }
                }
            }
            usvg::Node::Text(text) => {
                encode_group_to_scene(scene, text.flattened(), transform);
            }
        }
    }
}
