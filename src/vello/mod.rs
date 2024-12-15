use std::num::NonZeroUsize;

use vello::{kurbo::Affine, peniko::{BlendMode, Fill}};

use crate::{context::WgpuContext, utils};

pub struct Scene {
    pub vello_renderer: vello::Renderer,
    pub vello_scene: vello::Scene,
}

const svg_str: &str = include_str!("../../assets/Ghostscript_Tiger.svg");

impl Scene {
    pub fn new(wgpu_ctx: &WgpuContext) -> Self {
        let vello_renderer = vello::Renderer::new(
            &wgpu_ctx.device,
            vello::RendererOptions {
                surface_format: Some(wgpu::TextureFormat::Rgba8Unorm),
                use_cpu: false,
                antialiasing_support: vello::AaSupport::all(),
                num_init_threads: NonZeroUsize::new(1),
            },
        )
        .unwrap();
        let mut vello_scene = vello::Scene::new();
        let tree = usvg::Tree::from_str(svg_str, &usvg::Options::default())
            .unwrap();

        Self {
            vello_renderer,
            vello_scene,
        }
    }

    pub fn reset(&mut self) {
        self.vello_scene.reset();
    }

    pub fn render_svg(&mut self, svg: &usvg::Tree) {
        self.render_group(svg.root(), Affine::IDENTITY);
    }

    fn render_group(&mut self, group: &usvg::Group, transform: Affine) {
        for node in group.children() {
            let transform = transform * utils::to_affine(&node.abs_transform());
            match node {
                usvg::Node::Group(g) => {
                    let mut pushed_clip = false;
                    if let Some(clip_path) = g.clip_path() {
                        if let Some(usvg::Node::Path(clip_path)) =
                            clip_path.root().children().first()
                        {
                            // support clip-path with a single path
                            let local_path = utils::to_bez_path(clip_path);
                            self.vello_scene.push_layer(
                                BlendMode {
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

                    self.render_group(g, Affine::IDENTITY);

                    if pushed_clip {
                        self.vello_scene.pop_layer();
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
                                        usvg::FillRule::NonZero => Fill::NonZero,
                                        usvg::FillRule::EvenOdd => Fill::EvenOdd,
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
                            do_fill(&mut self.vello_scene);
                            do_stroke(&mut self.vello_scene);
                        }
                        usvg::PaintOrder::StrokeAndFill => {
                            do_stroke(&mut self.vello_scene);
                            do_fill(&mut self.vello_scene);
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
                            let Ok(decoded_image) = utils::decode_raw_raster_image(img.kind())
                            else {
                                // error_handler(scene, node);
                                continue;
                            };
                            let image = utils::into_image(decoded_image);
                            let image_ts = utils::to_affine(&img.abs_transform());
                            self.vello_scene.draw_image(&image, image_ts);
                        }
                        usvg::ImageKind::SVG(svg) => {
                            self.render_group(svg.root(), transform);
                        }
                    }
                }
                usvg::Node::Text(text) => {
                    self.render_group(text.flattened(), transform);
                }
            }
        }
    }

    pub fn render_to_texture(&mut self, wgpu_ctx: &WgpuContext, texture: &wgpu::TextureView) {
        self.vello_renderer
            .render_to_texture(
                &wgpu_ctx.device,
                &wgpu_ctx.queue,
                &self.vello_scene,
                texture,
                &vello::RenderParams {
                    base_color: vello::peniko::Color::TRANSPARENT,
                    width: 1920,
                    height: 1080,
                    antialiasing_method: vello::AaConfig::Msaa16,
                },
            )
            .unwrap();
    }
}
