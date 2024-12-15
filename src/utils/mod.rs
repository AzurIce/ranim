pub mod rate_functions;
pub mod wgpu;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use bevy_color::Srgba;
use glam::{vec2, vec3, Mat3, Vec2, Vec3};

use crate::{context::WgpuContext, interpolate::Interpolatable, rabject::RenderResource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(u128);

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl Id {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().as_u128())
    }
}

#[derive(Default)]
pub struct RenderResourceStorage {
    inner: HashMap<TypeId, Box<dyn Any>>,
}

impl RenderResourceStorage {
    pub fn get_or_init<P: RenderResource + 'static>(&mut self, ctx: &WgpuContext) -> &P {
        let id = std::any::TypeId::of::<P>();
        self.inner
            .entry(id)
            .or_insert_with(|| {
                let pipeline = P::new(ctx);
                Box::new(pipeline)
            })
            .downcast_ref::<P>()
            .unwrap()
    }
    pub fn get_or_init_mut<P: RenderResource + 'static>(&mut self, ctx: &WgpuContext) -> &mut P {
        let id = std::any::TypeId::of::<P>();
        self.inner
            .entry(id)
            .or_insert_with(|| {
                let pipeline = P::new(ctx);
                Box::new(pipeline)
            })
            .downcast_mut::<P>()
            .unwrap()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SubpathWidth {
    Inner(f32),
    Outer(f32),
    Middle(f32),
}

impl Default for SubpathWidth {
    fn default() -> Self {
        Self::Middle(1.0)
    }
}

/// Projects a 3D point onto a plane defined by a unit normal vector.
pub fn project(p: Vec3, unit_normal: Vec3) -> Vec3 {
    // trace!("projecting {:?} by {:?}", p, unit_normal);
    // trace!("dot: {:?}", unit_normal.dot(p));
    // trace!("res: {:?}", p - unit_normal * unit_normal.dot(p));
    p - unit_normal * unit_normal.dot(p)
}

pub fn generate_basis(unit_normal: Vec3) -> (Vec3, Vec3) {
    // trace!("generating basis for {:?}", unit_normal);
    let u = if unit_normal.x != 0.0 || unit_normal.y != 0.0 {
        vec3(-unit_normal.y, unit_normal.x, 0.0)
    } else {
        vec3(1.0, 0.0, 0.0)
    }
    .normalize();
    let v = unit_normal.cross(u).normalize();
    (u, v)
}

pub fn convert_to_2d(p: Vec3, origin: Vec3, basis: (Vec3, Vec3)) -> Vec2 {
    // trace!("converting {:?} by {:?} and {:?}", p, origin, basis);
    let p_local = p - origin;
    vec2(basis.0.dot(p_local), basis.1.dot(p_local))
}

pub fn convert_to_3d(p: Vec2, origin: Vec3, basis: (Vec3, Vec3)) -> Vec3 {
    origin + basis.0 * p.x + basis.1 * p.y
}

pub fn rotation_between_vectors(v1: Vec3, v2: Vec3) -> Mat3 {
    // trace!("rotation_between_vectors: v1: {:?}, v2: {:?}", v1, v2);

    if (v2 - v1).length() < f32::EPSILON {
        return Mat3::IDENTITY;
    }
    let mut axis = v1.cross(v2);
    if axis.length() < f32::EPSILON {
        axis = v1.cross(Vec3::Y);
    }
    if axis.length() < f32::EPSILON {
        axis = v1.cross(Vec3::Z);
    }
    // trace!("axis: {:?}", axis);

    let angle = angle_between_vectors(v1, v2);
    // trace!("angle: {:?}", angle);
    Mat3::from_axis_angle(axis, angle)
}

pub fn angle_between_vectors(v1: Vec3, v2: Vec3) -> f32 {
    if v1.length() == 0.0 || v2.length() == 0.0 {
        return 0.0;
    }

    (v1.dot(v2) / (v1.length() * v2.length()))
        .clamp(-1.0, 1.0)
        .acos()
}

pub fn resize_preserving_order<T: Clone>(vec: &[T], new_len: usize) -> Vec<T> {
    let indices = (0..new_len).map(|i| i * vec.len() / new_len);
    indices.map(|i| vec[i].clone()).collect()
}

pub fn extend_with_last<T: Clone + Default>(vec: &mut Vec<T>, new_len: usize) {
    let v = vec![vec.last().cloned().unwrap_or_default(); new_len - vec.len()];
    vec.extend(v)
}

/// Returns the point on a quadratic bezier curve at the given parameter.
pub fn point_on_quadratic_bezier<T: Interpolatable>(points: &[T; 3], t: f32) -> T {
    let t = t.clamp(0.0, 1.0);
    let p1 = points[0].lerp(&points[1], t);
    let p2 = points[1].lerp(&points[2], t);
    p1.lerp(&p2, t)
}

/// Returns the control points of the given part of a quadratic bezier curve.
pub fn partial_quadratic_bezier<T: Interpolatable>(points: &[T; 3], a: f32, b: f32) -> [T; 3] {
    let a = a.clamp(0.0, 1.0);
    let b = b.clamp(0.0, 1.0);

    let h0 = point_on_quadratic_bezier(points, a);
    let h2 = point_on_quadratic_bezier(points, b);

    let h1_prime = points[1].lerp(&points[2], a);
    let end_prop = (b - a) / (1.0 - a);
    let h1 = h0.lerp(&h1_prime, end_prop);
    [h0, h1, h2]
}

impl Interpolatable for Srgba {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            red: self.red.lerp(&other.red, t),
            green: self.green.lerp(&other.green, t),
            blue: self.blue.lerp(&other.blue, t),
            alpha: self.alpha.lerp(&other.alpha, t),
        }
    }
}

// Copyright 2023 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use vello::kurbo::{Affine, BezPath, Point, Rect, Stroke};
use vello::peniko::{Blob, Brush, Color, Fill, Image};
use vello::Scene;

pub fn to_affine(ts: &usvg::Transform) -> Affine {
    let usvg::Transform {
        sx,
        kx,
        ky,
        sy,
        tx,
        ty,
    } = ts;
    Affine::new([sx, kx, ky, sy, tx, ty].map(|&x| f64::from(x)))
}

pub fn to_stroke(stroke: &usvg::Stroke) -> Stroke {
    let mut conv_stroke = Stroke::new(stroke.width().get() as f64)
        .with_caps(match stroke.linecap() {
            usvg::LineCap::Butt => vello::kurbo::Cap::Butt,
            usvg::LineCap::Round => vello::kurbo::Cap::Round,
            usvg::LineCap::Square => vello::kurbo::Cap::Square,
        })
        .with_join(match stroke.linejoin() {
            usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => vello::kurbo::Join::Miter,
            usvg::LineJoin::Round => vello::kurbo::Join::Round,
            usvg::LineJoin::Bevel => vello::kurbo::Join::Bevel,
        })
        .with_miter_limit(stroke.miterlimit().get() as f64);
    if let Some(dash_array) = stroke.dasharray().as_ref() {
        conv_stroke = conv_stroke.with_dashes(
            stroke.dashoffset() as f64,
            dash_array.iter().map(|x| *x as f64),
        );
    }
    conv_stroke
}

pub fn to_bez_path(path: &usvg::Path) -> BezPath {
    let mut local_path = BezPath::new();
    // The semantics of SVG paths don't line up with `BezPath`; we
    // must manually track initial points
    let mut just_closed = false;
    let mut most_recent_initial = (0., 0.);
    for elt in path.data().segments() {
        match elt {
            usvg::tiny_skia_path::PathSegment::MoveTo(p) => {
                if std::mem::take(&mut just_closed) {
                    local_path.move_to(most_recent_initial);
                }
                most_recent_initial = (p.x.into(), p.y.into());
                local_path.move_to(most_recent_initial);
            }
            usvg::tiny_skia_path::PathSegment::LineTo(p) => {
                if std::mem::take(&mut just_closed) {
                    local_path.move_to(most_recent_initial);
                }
                local_path.line_to(Point::new(p.x as f64, p.y as f64));
            }
            usvg::tiny_skia_path::PathSegment::QuadTo(p1, p2) => {
                if std::mem::take(&mut just_closed) {
                    local_path.move_to(most_recent_initial);
                }
                local_path.quad_to(
                    Point::new(p1.x as f64, p1.y as f64),
                    Point::new(p2.x as f64, p2.y as f64),
                );
            }
            usvg::tiny_skia_path::PathSegment::CubicTo(p1, p2, p3) => {
                if std::mem::take(&mut just_closed) {
                    local_path.move_to(most_recent_initial);
                }
                local_path.curve_to(
                    Point::new(p1.x as f64, p1.y as f64),
                    Point::new(p2.x as f64, p2.y as f64),
                    Point::new(p3.x as f64, p3.y as f64),
                );
            }
            usvg::tiny_skia_path::PathSegment::Close => {
                just_closed = true;
                local_path.close_path();
            }
        }
    }

    local_path
}

pub fn into_image(image: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>) -> Image {
    let (width, height) = (image.width(), image.height());
    let image_data: Vec<u8> = image.into_vec();
    Image::new(
        Blob::new(std::sync::Arc::new(image_data)),
        vello::peniko::Format::Rgba8,
        width,
        height,
    )
}

pub fn to_brush(paint: &usvg::Paint, opacity: usvg::Opacity) -> Option<(Brush, Affine)> {
    match paint {
        usvg::Paint::Color(color) => Some((
            Brush::Solid(Color::from_rgba8(
                color.red,
                color.green,
                color.blue,
                opacity.to_u8(),
            )),
            Affine::IDENTITY,
        )),
        usvg::Paint::LinearGradient(gr) => {
            let stops: Vec<vello::peniko::ColorStop> = gr
                .stops()
                .iter()
                .map(|stop| {
                    let mut cstop = vello::peniko::ColorStop::from((
                        stop.offset().get(),
                        Color::from_rgba8(
                            stop.color().red,
                            stop.color().green,
                            stop.color().blue,
                            (stop.opacity() * opacity).to_u8(),
                        ),
                    ));
                    cstop
                })
                .collect();
            let start = Point::new(gr.x1() as f64, gr.y1() as f64);
            let end = Point::new(gr.x2() as f64, gr.y2() as f64);
            let arr = [
                gr.transform().sx,
                gr.transform().ky,
                gr.transform().kx,
                gr.transform().sy,
                gr.transform().tx,
                gr.transform().ty,
            ]
            .map(f64::from);
            let transform = Affine::new(arr);
            let gradient =
                vello::peniko::Gradient::new_linear(start, end).with_stops(stops.as_slice());
            Some((Brush::Gradient(gradient), transform))
        }
        usvg::Paint::RadialGradient(gr) => {
            let stops: Vec<vello::peniko::ColorStop> = gr
                .stops()
                .iter()
                .map(|stop| {
                    vello::peniko::ColorStop::from((
                        stop.offset().get(),
                        Color::from_rgba8(
                            stop.color().red,
                            stop.color().green,
                            stop.color().blue,
                            (stop.opacity() * opacity).to_u8(),
                        ),
                    ))
                })
                .collect();

            let start_center = Point::new(gr.cx() as f64, gr.cy() as f64);
            let end_center = Point::new(gr.fx() as f64, gr.fy() as f64);
            let start_radius = 0_f32;
            let end_radius = gr.r().get();
            let arr = [
                gr.transform().sx,
                gr.transform().ky,
                gr.transform().kx,
                gr.transform().sy,
                gr.transform().tx,
                gr.transform().ty,
            ]
            .map(f64::from);
            let transform = Affine::new(arr);
            let gradient = vello::peniko::Gradient::new_two_point_radial(
                start_center,
                start_radius,
                end_center,
                end_radius,
            )
            .with_stops(stops.as_slice());
            Some((Brush::Gradient(gradient), transform))
        }
        usvg::Paint::Pattern(_) => None,
    }
}

/// Error handler function for [`super::append_tree_with`] which draws a transparent red box
/// instead of unsupported SVG features
pub fn default_error_handler(scene: &mut Scene, node: &usvg::Node) {
    let bb = node.bounding_box();
    let rect = Rect {
        x0: bb.left() as f64,
        y0: bb.top() as f64,
        x1: bb.right() as f64,
        y1: bb.bottom() as f64,
    };
    scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        Color::from_rgba8(255, 0, 0, 128),
        None,
        &rect,
    );
}

pub fn decode_raw_raster_image(
    img: &usvg::ImageKind,
) -> Result<image::RgbaImage, image::ImageError> {
    let res = match img {
        usvg::ImageKind::JPEG(data) => {
            image::load_from_memory_with_format(data, image::ImageFormat::Jpeg)
        }
        usvg::ImageKind::PNG(data) => {
            image::load_from_memory_with_format(data, image::ImageFormat::Png)
        }
        usvg::ImageKind::GIF(data) => {
            image::load_from_memory_with_format(data, image::ImageFormat::Gif)
        }
        usvg::ImageKind::WEBP(data) => {
            image::load_from_memory_with_format(data, image::ImageFormat::WebP)
        }
        usvg::ImageKind::SVG(_) => unreachable!(),
    }?
    .into_rgba8();
    Ok(res)
}
