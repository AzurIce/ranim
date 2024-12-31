use bevy_color::LinearRgba;
use vello::{
    kurbo::{self, Affine},
    peniko::{self, color::AlphaColor, Brush},
};

use crate::{
    rabject::Blueprint,
    scene::{canvas::camera::CanvasCamera, Entity},
};

#[derive(Clone, Debug)]
pub struct BezPath {
    pub inner: kurbo::BezPath,
    pub stroke: Option<StrokeOptions>,
    pub fill: Option<FillOptions>,
}

#[derive(Clone, Debug)]
pub struct StrokeOptions {
    pub style: kurbo::Stroke,
    pub transform: Option<Affine>,
    pub brush: Brush,
}

impl Default for StrokeOptions {
    fn default() -> Self {
        Self {
            style: kurbo::Stroke::default(),
            transform: None,
            brush: Brush::Solid(peniko::color::palette::css::RED),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FillOptions {
    pub style: peniko::Fill,
    pub transform: Option<Affine>,
    pub brush: Brush,
}

impl Default for FillOptions {
    fn default() -> Self {
        Self {
            style: peniko::Fill::NonZero,
            transform: None,
            brush: Brush::Solid(peniko::color::palette::css::BLUE),
        }
    }
}

pub struct ArcBezPathBlueprint {
    pub angle: f32,
    pub radius: f32,
    pub x_rotation: f32,
    pub stroke_width: f32,
}

impl ArcBezPathBlueprint {
    pub fn with_stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }
}

impl Default for ArcBezPathBlueprint {
    fn default() -> Self {
        Self {
            angle: 0.0,
            radius: 0.0,
            x_rotation: 0.0,
            stroke_width: 10.0,
        }
    }
}

impl Blueprint<BezPath> for ArcBezPathBlueprint {
    fn build(self) -> BezPath {
        // when x_rotation is 0.0, the arc starts from (radius, 0.0) and goes clockwise
        let start = (
            self.radius * self.x_rotation.cos(),
            self.radius * self.x_rotation.sin(),
        );

        let path = kurbo::BezPath::from_vec(
            [kurbo::PathEl::MoveTo(
                (start.0 as f64, start.1 as f64).into(),
            )]
            .into_iter()
            .chain(
                kurbo::Arc::new(
                    (0.0, 0.0),
                    (self.radius as f64, self.radius as f64),
                    0.0,
                    self.angle as f64,
                    0.0, // std::f64::consts::PI / 2.0,
                )
                .append_iter(0.1),
            )
            .collect(),
        );

        let stroke = Some(StrokeOptions::default());
        let fill = Some(FillOptions::default());

        BezPath {
            inner: path,
            stroke,
            fill,
        }
    }
}

impl BezPath {
    pub fn arc(angle: f32, radius: f32) -> ArcBezPathBlueprint {
        ArcBezPathBlueprint {
            angle,
            radius,
            ..Default::default()
        }
    }
}

impl BezPath {
    pub fn set_stroke_width(&mut self, width: f32) {
        if let Some(stroke) = &mut self.stroke {
            stroke.style.width = width as f64;
        }
    }
    pub fn set_stroke_color(&mut self, color: LinearRgba) {
        if let Some(stroke) = &mut self.stroke {
            stroke.brush = peniko::Brush::Solid(AlphaColor::new([
                color.red,
                color.green,
                color.blue,
                color.alpha,
            ]));
        }
    }
    pub fn set_stroke_alpha(&mut self, alpha: f32) {
        if let Some(mut stroke) = self.stroke.take() {
            stroke.brush = stroke.brush.with_alpha(alpha);
            self.stroke = Some(stroke);
        }
    }
    /* pub fn set_fill_color(&mut self, color: Option<LinearRgba>) {
        self.fill =
            color.map(|c| peniko::Brush::Solid(AlphaColor::new([c.red, c.green, c.blue, c.alpha])));
    }
    pub fn set_fill_alpha(&mut self, alpha: f32) {
        if let Some(fill) = self.fill.take() {
            self.fill = Some(fill.with_alpha(alpha));
        }
    }
    pub fn set_alpha(&mut self, alpha: f32) {
        self.set_stroke_alpha(alpha);
        self.set_fill_alpha(alpha);
    } */
    // transforms
    pub fn apply_affine(&mut self, affine: kurbo::Affine) {
        self.inner.apply_affine(affine);
    }
    pub fn shift(&mut self, shift: (f32, f32)) {
        self.inner
            .apply_affine(kurbo::Affine::translate((shift.0 as f64, shift.1 as f64)));
    }
    pub fn rotate(&mut self, angle: f32) {
        self.inner.apply_affine(kurbo::Affine::rotate(angle as f64));
    }
    pub fn scale(&mut self, scale: f32) {
        self.inner.apply_affine(kurbo::Affine::scale(scale as f64));
    }
}

impl Entity for BezPath {
    type Renderer = CanvasCamera;

    fn tick(&mut self, _dt: f32) {}
    fn extract(&mut self) {}
    fn prepare(&mut self, _ctx: &crate::context::RanimContext) {}
    fn render(&mut self, _ctx: &mut crate::context::RanimContext, renderer: &mut Self::Renderer) {
        if let Some(fill_options) = self.fill.as_ref() {
            renderer.vello_scene.fill(
                fill_options.style,
                kurbo::Affine::IDENTITY,
                &fill_options.brush,
                fill_options.transform,
                &self.inner,
            );
        }
        if let Some(stroke_options) = self.stroke.as_ref() {
            renderer.vello_scene.stroke(
                &stroke_options.style,
                kurbo::Affine::IDENTITY,
                &stroke_options.brush,
                stroke_options.transform,
                &self.inner,
            );
        }
    }
}
