use std::fs;

use usvg::{Options, Tree};

use crate::{canvas::camera::CanvasCamera, scene::entity::Entity, utils};

#[derive(Clone)]
pub struct SvgMobject {
    tree: usvg::Tree,
    scene: vello::Scene,
}

impl SvgMobject {
    pub fn from_path(path: &str) -> Self {
        let str = fs::read_to_string(path).unwrap();
        let tree = Tree::from_str(&str, &Options::default()).unwrap();
        Self::from_tree(tree)
    }

    pub fn from_tree(tree: Tree) -> Self {
        let scene = vello::Scene::new();
        Self { tree, scene }
    }
}

#[allow(unused)]
impl Entity for SvgMobject {
    type Renderer = CanvasCamera;

    fn tick(&mut self, dt: f32) {}
    fn extract(&mut self) {}
    fn prepare(&mut self, ctx: &crate::context::RanimContext) {}
    fn render(&mut self, ctx: &mut crate::context::RanimContext, renderer: &mut Self::Renderer) {
        self.scene.reset();
        encode_group_to_scene(
            &mut self.scene,
            &self.tree.root(),
            vello::kurbo::Affine::IDENTITY,
        );
        renderer.vello_scene.append(&self.scene, None);
    }
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
                let local_path = utils::to_bez_path(path);

                let do_fill = |scene: &mut vello::Scene| {
                    if let Some(fill) = &path.fill() {
                        if let Some((brush, brush_transform)) =
                            utils::to_brush(fill.paint(), fill.opacity())
                        {
                            scene.fill(
                                match fill.rule() {
                                    usvg::FillRule::NonZero => vello::peniko::Fill::NonZero,
                                    usvg::FillRule::EvenOdd => vello::peniko::Fill::EvenOdd,
                                },
                                transform,
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
                                transform,
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
