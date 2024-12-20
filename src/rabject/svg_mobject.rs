use bevy_color::{Alpha, Srgba};
use glam::vec3;
use log::warn;
use usvg::{
    tiny_skia_path::{PathSegment, Point},
    Group, Node, Options, Paint, Path, Transform, Tree,
};

use crate::{context::WgpuContext, utils::{self, RenderResourceStorage}};

use super::{
    vpath::{
        blueprint::VPathBuilder,
        primitive::{ExtractedVPath, VPathPrimitive},
        VPath,
    },
    Blueprint, Primitive, Rabject,
};

#[derive(Debug, Clone)]
pub enum SvgNode {
    Path(Vec<VPath>),
    Group(Vec<SvgNode>),
}

impl SvgNode {
    // pub fn path(path: &Path, transform: Option<Transform>) -> Self {
    //     // println!("{:?}", path);
    //     let segments = path.data().segments().into_iter().collect::<Vec<_>>();

    //     let point2vec3 = |p: &Point| {
    //         let mut p = p.clone();
    //         if let Some(transform) = transform {
    //             transform.map_point(&mut p);
    //         }
    //         vec3(p.x, p.y, 0.0)
    //     };

    //     let PathSegment::MoveTo(start) = segments[0] else {
    //         panic!("Path must start with a move_to segment");
    //     };
    //     let mut builder = VPathBuilder::start(point2vec3(&start));

    //     let mut paths = vec![];
    //     for segment in segments.get(1..).unwrap() {
    //         // println!("{:?}", segment);
    //         builder = match segment {
    //             PathSegment::MoveTo(p) => {
    //                 warn!("should not have move to in the middle of the path");
    //                 // unreachable!("should not have move to in the middle of the path")
    //                 paths.push(builder.build());
    //                 VPathBuilder::start(point2vec3(p))
    //             }
    //             PathSegment::LineTo(p) => builder.line_to(point2vec3(p)),
    //             PathSegment::QuadTo(h, p) => builder.quad_to(point2vec3(p), point2vec3(h)),
    //             PathSegment::CubicTo(h0, h1, p) => {
    //                 builder.cubic_to(point2vec3(p), point2vec3(h0), point2vec3(h1))
    //             }
    //             PathSegment::Close => builder.close(),
    //         };
    //     }
    //     paths.push(builder.build());

    //     paths.iter_mut().for_each(|p| {
    //         p.paint_order = path.paint_order();
    //     });

    //     // Set stroke styles
    //     if let Some(stroke) = path.stroke() {
    //         paths.iter_mut().for_each(|p| {
    //             p.set_stroke_width(stroke.width().get());
    //             if let Paint::Color(color) = stroke.paint() {
    //                 let opacity = stroke.opacity().get();
    //                 p.set_stroke_color(
    //                     Srgba::rgb_u8(color.red, color.green, color.blue)
    //                         .with_alpha(opacity)
    //                         .into(),
    //                 );
    //             }
    //         });
    //     } else {
    //         paths.iter_mut().for_each(|p| {
    //             p.set_stroke_color(Srgba::new(0.0, 0.0, 0.0, 0.0).into());
    //         });
    //     }

    //     // Set fill styles
    //     if let Some(fill) = path.fill() {
    //         if let Paint::Color(color) = fill.paint() {
    //             let opacity = fill.opacity().get();
    //             paths.iter_mut().for_each(|p| {
    //                 p.set_fill_color(
    //                     Srgba::rgb_u8(color.red, color.green, color.blue)
    //                         .with_alpha(opacity)
    //                         .into(),
    //                 );
    //             });
    //         }
    //     } else {
    //         paths.iter_mut().for_each(|p| {
    //             p.set_fill_color(Srgba::new(0.0, 0.0, 0.0, 0.0).into());
    //         });
    //     }

    //     Self::Path(paths)
    // }

    // pub fn group(group: &Group) -> Self {
    //     let children = group
    //         .children()
    //         .iter()
    //         .filter_map(|child| match child {
    //             Node::Group(group) => Some(SvgNode::group(group)),
    //             Node::Path(path) => {
    //                 if path.is_visible() {
    //                     Some(SvgNode::path(path, Some(group.abs_transform())))
    //                 } else {
    //                     None
    //                 }
    //             }
    //             Node::Image(image) => {
    //                 unimplemented!()
    //             }
    //             Node::Text(text) => {
    //                 unimplemented!()
    //             }
    //         })
    //         .collect();
    //     Self::Group(children)
    // }

    // pub fn extract(&self) -> SvgRenderData {
    // match self {
    //     SvgNode::Path(paths) => {
    //         SvgRenderData::Path(paths.iter().map(|path| path.extract()).collect())
    //     }
    //     SvgNode::Group(children) => {
    //         SvgRenderData::Group(children.iter().map(|child| child.extract()).collect())
    //     }
    // }
    // }
}

#[derive(Debug, Clone)]
pub struct SvgMobject {
    tree: usvg::Tree,
}

impl SvgMobject {
    pub fn from_tree(tree: Tree) -> Self {
        Self { tree }
    }
}

impl Rabject for SvgMobject {
    type RenderData = SvgRenderData;
    type RenderResource = SvgPrimitive;
    fn extract(&self) -> Self::RenderData {
        SvgRenderData { tree: self.tree.clone() }
    }
}

pub struct SvgRenderData {
    tree: Tree,
}

pub struct SvgPrimitive {
    scene: vello::Scene,
}

impl Primitive for SvgPrimitive {
    type Data = SvgRenderData;
    fn init(_wgpu_ctx: &WgpuContext, data: &Self::Data) -> Self {
        let mut scene = vello::Scene::new();
        encode_group_to_scene(&mut scene, &data.tree.root(), vello::kurbo::Affine::IDENTITY);

        Self { scene }
    }

    fn update(&mut self, _wgpu_ctx: &WgpuContext, data: &Self::Data) {
        self.scene.reset();
        encode_group_to_scene(&mut self.scene, &data.tree.root(), vello::kurbo::Affine::IDENTITY);
    }

    fn render(
        &self,
        wgpu_ctx: &WgpuContext,
        pipelines: &mut RenderResourceStorage,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        uniforms_bind_group: &wgpu::BindGroup,
    ) {
        // TODO: render inner render_resource and vello scene
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

// impl Primitive for SvgPrimitive {
//     type Data = SvgRenderData;
//     fn init(wgpu_ctx: &WgpuContext, data: &Self::Data) -> Self {
//         match data {
//             SvgRenderData::Path(paths) => SvgPrimitive::Path(
//                 paths
//                     .iter()
//                     .map(|path| VPathPrimitive::init(wgpu_ctx, path))
//                     .collect(),
//             ),
//             SvgRenderData::Group(children) => SvgPrimitive::Group(
//                 children
//                     .iter()
//                     .map(|child| Self::init(wgpu_ctx, child))
//                     .collect(),
//             ),
//         }
//     }
//     fn update(&mut self, wgpu_ctx: &WgpuContext, data: &Self::Data) {
//         match self {
//             SvgPrimitive::Path(paths) => {
//                 let SvgRenderData::Path(data) = data else {
//                     panic!("expected path data");
//                 };
//                 paths
//                     .iter_mut()
//                     .zip(data)
//                     .for_each(|(path, data)| path.update(wgpu_ctx, data));
//             }
//             SvgPrimitive::Group(children) => children
//                 .iter_mut()
//                 .for_each(|child| child.update(wgpu_ctx, data)),
//         }
//     }
//     fn render(
//         &self,
//         wgpu_ctx: &crate::context::WgpuContext,
//         pipelines: &mut crate::utils::RenderResourceStorage,
//         multisample_view: &wgpu::TextureView,
//         target_view: &wgpu::TextureView,
//         depth_view: &wgpu::TextureView,
//         uniforms_bind_group: &wgpu::BindGroup,
//     ) {
//         match self {
//             SvgPrimitive::Path(paths) => paths.iter().for_each(|path| {
//                 path.render(
//                     wgpu_ctx,
//                     pipelines,
//                     multisample_view,
//                     target_view,
//                     depth_view,
//                     uniforms_bind_group,
//                 )
//             }),
//             SvgPrimitive::Group(children) => children.iter().for_each(|child| {
//                 child.render(
//                     wgpu_ctx,
//                     pipelines,
//                     multisample_view,
//                     target_view,
//                     depth_view,
//                     uniforms_bind_group,
//                 )
//             }),
//         }
//     }
// }

// impl Rabject for SvgMobject {
//     type RenderData = SvgRenderData;
//     type RenderResource = SvgPrimitive;
//     fn extract(&self) -> Self::RenderData {
//         self.root.extract()
//     }
// }

// #[cfg(test)]
// mod test {
//     use usvg::tiny_skia_path::{self, Stroke};

//     #[test]
//     fn foo() {
//         let mut path = tiny_skia_path::PathBuilder::new();
//         path.cubic_to(10.0, 10.0, 20.0, 10.0, 30.0, 0.0);
//         let path = path.finish().unwrap();

//         for segment in path.segments() {
//             println!("{:?}", segment);
//         }
//         println!("{:?}", path.points());
//         let stroke = path.stroke(&Stroke::default(), 1.0).unwrap();
//         for segment in stroke.segments() {
//             println!("{:?}", segment);
//         }
//         println!("{:?}", stroke.points());

//         // let tree = Tree::from_str(TEST_SVG, &Options::default()).unwrap();
//         // println!("{:?}", tree.root().children());
//     }

//     const TEST_SVG: &str = include_str!("../../assets/test.svg");
// }
