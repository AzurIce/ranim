use std::{
    any::Any,
    collections::HashMap,
    panic::{AssertUnwindSafe, catch_unwind},
};

use dyn_clone::DynClone;
use egui::{
    Align2, Color32, CornerRadius, DragValue, FontId, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2,
    pos2, vec2,
};
use ranim_core::{
    Extract,
    color::{AlphaColor, Srgb},
    core_item::CoreItem,
    glam::{DMat4, DVec2, DVec3, Vec3, dvec2, dvec3, vec3},
    traits::{Aabb, FillColor, ScaleTransform, ShiftTransform, StrokeColor, StrokeWidth},
};
use ranim_items::{
    mesh::{MeshItem, Sphere, Surface},
    vitem::{
        VItem,
        geometry::{
            Arc, ArcBetweenPoints, Circle, Ellipse, EllipticArc, Line, Parallelogram, Polygon,
            Rectangle, RegularPolygon, Square,
        },
        svg::SvgItem,
        text::TextItem,
        typst::TypstText,
    },
};

use crate::model::MIN_OBJECT_SIZE;

const DEFAULT_SHAPE_STROKE_WIDTH: f32 = 0.02;
const EMPHASIS_STROKE_WIDTH: f32 = 0.04;

macro_rules! slide_object_descriptor {
    ($name:ident, $type_id:literal, $display_name:literal, $create_default:expr) => {
        pub static $name: SlideObjectDescriptor = SlideObjectDescriptor {
            type_id: $type_id,
            display_name: $display_name,
            create_default: $create_default,
        };
    };
}

pub struct SlideObjectDescriptor {
    pub type_id: &'static str,
    pub display_name: &'static str,
    pub create_default: fn() -> Box<dyn SlideObject>,
}

pub struct SlideObjectRegistry {
    descriptors: Vec<&'static SlideObjectDescriptor>,
    by_type_id: HashMap<&'static str, usize>,
}

impl SlideObjectRegistry {
    pub fn builtin() -> Self {
        let mut registry = Self {
            descriptors: Vec::new(),
            by_type_id: HashMap::new(),
        };
        registry.register(&RECTANGLE_DESCRIPTOR);
        registry.register(&TEXT_DESCRIPTOR);
        registry.register(&CIRCLE_DESCRIPTOR);
        registry.register(&SQUARE_DESCRIPTOR);
        registry.register(&ELLIPSE_DESCRIPTOR);
        registry.register(&LINE_DESCRIPTOR);
        registry.register(&ARC_DESCRIPTOR);
        registry.register(&ARC_BETWEEN_POINTS_DESCRIPTOR);
        registry.register(&ELLIPTIC_ARC_DESCRIPTOR);
        registry.register(&PARALLELOGRAM_DESCRIPTOR);
        registry.register(&POLYGON_DESCRIPTOR);
        registry.register(&REGULAR_POLYGON_DESCRIPTOR);
        registry.register(&SVG_DESCRIPTOR);
        registry.register(&TYPST_TEXT_DESCRIPTOR);
        registry.register(&RAW_VITEM_DESCRIPTOR);
        registry.register(&SPHERE_DESCRIPTOR);
        registry.register(&SURFACE_DESCRIPTOR);
        registry.register(&RAW_MESH_DESCRIPTOR);
        registry
    }

    pub fn register(&mut self, descriptor: &'static SlideObjectDescriptor) {
        if self.by_type_id.contains_key(descriptor.type_id) {
            return;
        }

        self.by_type_id
            .insert(descriptor.type_id, self.descriptors.len());
        self.descriptors.push(descriptor);
    }

    pub fn descriptors(&self) -> &[&'static SlideObjectDescriptor] {
        &self.descriptors
    }

    #[allow(dead_code)]
    pub fn get(&self, type_id: &str) -> Option<&'static SlideObjectDescriptor> {
        self.by_type_id
            .get(type_id)
            .map(|idx| self.descriptors[*idx])
    }
}

#[derive(Default)]
pub struct InspectorResponse {
    pub changed: bool,
    pub request_repaint: bool,
    pub request_relayout: bool,
}

impl InspectorResponse {
    pub fn changed(changed: bool) -> Self {
        Self {
            changed,
            request_repaint: changed,
            request_relayout: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PaintCtx {
    pub canvas_rect: Rect,
    pub scale: f32,
    pub frame_min: Pos2,
    pub frame_max: Pos2,
}

impl PaintCtx {
    pub fn scene_rect_to_screen(&self, rect: Rect) -> Rect {
        Rect::from_min_max(
            self.scene_pos_to_screen(pos2(rect.min.x, rect.max.y)),
            self.scene_pos_to_screen(pos2(rect.max.x, rect.min.y)),
        )
    }

    pub fn scene_pos_to_screen(&self, pos: Pos2) -> Pos2 {
        pos2(
            self.canvas_rect.min.x + (pos.x - self.frame_min.x) * self.scale,
            self.canvas_rect.min.y + (self.frame_max.y - pos.y) * self.scale,
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct InspectorCtx;

#[derive(Clone, Copy, Debug)]
pub struct RenderCtx;

pub trait SlideObject: Any + DynClone {
    fn descriptor(&self) -> &'static SlideObjectDescriptor;
    fn bounds(&self) -> Rect;
    fn bounds3(&self) -> [DVec3; 2] {
        let bounds = self.bounds();
        [
            dvec3(bounds.min.x as f64, bounds.min.y as f64, 0.0),
            dvec3(bounds.max.x as f64, bounds.max.y as f64, 0.0),
        ]
    }
    fn position(&self) -> DVec3 {
        ordered_aabb(self.bounds3())[0]
    }
    fn translate(&mut self, delta: Vec2);
    fn translate3(&mut self, delta: DVec3) {
        self.translate(vec2(delta.x as f32, delta.y as f32));
    }
    fn set_position(&mut self, pos: Pos2);
    fn set_position3(&mut self, pos: DVec3) {
        let current = self.position();
        self.translate3(pos - current);
    }
    fn set_size(&mut self, size: Vec2);
    fn inspector_ui(&mut self, ui: &mut Ui, ctx: &mut InspectorCtx) -> InspectorResponse;
    fn paint_preview(&self, ui: &Ui, ctx: &PaintCtx);
    fn extract_core_items(&self, ctx: &RenderCtx, out: &mut Vec<CoreItem>);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn hit_test(&self, scene_pos: Pos2) -> bool {
        self.bounds().contains(scene_pos)
    }
}

dyn_clone::clone_trait_object!(SlideObject);

impl std::fmt::Debug for dyn SlideObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlideObject")
            .field("type_id", &self.descriptor().type_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub struct TextObject {
    pub rect: Rect,
    pub z: f64,
    pub text: String,
    pub font_size: f32,
    pub fill: Color32,
}

impl TextObject {
    pub fn new_default() -> Self {
        Self {
            rect: Rect::from_min_size(pos2(-1.8, -0.3), vec2(3.6, 0.7)),
            z: 0.0,
            text: "Text".to_owned(),
            font_size: 0.5,
            fill: Color32::from_rgb(30, 35, 42),
        }
    }
}

impl SlideObject for TextObject {
    fn descriptor(&self) -> &'static SlideObjectDescriptor {
        &TEXT_DESCRIPTOR
    }

    fn bounds(&self) -> Rect {
        self.rect
    }

    fn bounds3(&self) -> [DVec3; 2] {
        [
            dvec3(self.rect.min.x as f64, self.rect.min.y as f64, self.z),
            dvec3(self.rect.max.x as f64, self.rect.max.y as f64, self.z),
        ]
    }

    fn translate(&mut self, delta: Vec2) {
        self.rect = self.rect.translate(delta);
    }

    fn translate3(&mut self, delta: DVec3) {
        self.rect = self.rect.translate(vec2(delta.x as f32, delta.y as f32));
        self.z += delta.z;
    }

    fn set_position(&mut self, pos: Pos2) {
        self.rect = Rect::from_min_size(pos, self.rect.size());
    }

    fn set_size(&mut self, size: Vec2) {
        self.rect = Rect::from_min_size(self.rect.min, clamp_size(size));
    }

    fn inspector_ui(&mut self, ui: &mut Ui, _ctx: &mut InspectorCtx) -> InspectorResponse {
        let mut changed = false;
        ui.label("Text");
        changed |= ui.text_edit_multiline(&mut self.text).changed();
        changed |= ui
            .add(
                DragValue::new(&mut self.font_size)
                    .speed(0.01)
                    .range(0.05..=4.0)
                    .prefix("Size "),
            )
            .changed();
        ui.label("Fill");
        changed |= ui.color_edit_button_srgba(&mut self.fill).changed();
        InspectorResponse::changed(changed)
    }

    fn paint_preview(&self, ui: &Ui, ctx: &PaintCtx) {
        let rect = ctx.scene_rect_to_screen(self.rect);
        ui.painter().text(
            rect.left_top(),
            Align2::LEFT_TOP,
            &self.text,
            FontId::proportional(self.font_size * ctx.scale),
            self.fill,
        );
    }

    fn extract_core_items(&self, _ctx: &RenderCtx, out: &mut Vec<CoreItem>) {
        let mut item = TextItem::new(&self.text, self.font_size as f64);
        item.set_fill_color(egui_to_ranim_color(self.fill));
        item.shift(dvec3(
            self.rect.min.x as f64,
            self.rect.min.y as f64,
            self.z,
        ));
        item.extract_into(out);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl SlideObject for Rectangle {
    fn descriptor(&self) -> &'static SlideObjectDescriptor {
        &RECTANGLE_DESCRIPTOR
    }

    fn bounds(&self) -> Rect {
        Rect::from_min_size(
            pos2(self.p0.x as f32, self.p0.y as f32),
            vec2(self.size.x.abs() as f32, self.size.y.abs() as f32),
        )
    }

    fn bounds3(&self) -> [DVec3; 2] {
        self.aabb()
    }

    fn translate(&mut self, delta: Vec2) {
        self.p0.x += delta.x as f64;
        self.p0.y += delta.y as f64;
    }

    fn translate3(&mut self, delta: DVec3) {
        self.shift(delta);
    }

    fn set_position(&mut self, pos: Pos2) {
        self.p0.x = pos.x as f64;
        self.p0.y = pos.y as f64;
    }

    fn set_size(&mut self, size: Vec2) {
        let size = clamp_size(size);
        self.size = dvec2(size.x as f64, size.y as f64);
    }

    fn inspector_ui(&mut self, ui: &mut Ui, _ctx: &mut InspectorCtx) -> InspectorResponse {
        let mut changed = false;
        changed |= dvec3_ui(ui, "P0", &mut self.p0);
        changed |= dvec2_ui(ui, "Size", &mut self.size);
        self.size.x = self.size.x.abs().max(MIN_OBJECT_SIZE as f64);
        self.size.y = self.size.y.abs().max(MIN_OBJECT_SIZE as f64);
        changed |= axes_ui(ui, &mut self.axes);
        changed |= fill_color_ui(ui, &mut self.fill_rgba);
        changed |= stroke_color_ui(ui, &mut self.stroke_rgba);
        changed |= stroke_width_ui(ui, &mut self.stroke_width);
        InspectorResponse::changed(changed)
    }

    fn paint_preview(&self, ui: &Ui, ctx: &PaintCtx) {
        let rect = ctx.scene_rect_to_screen(self.bounds());
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(2),
            ranim_to_egui_color(self.fill_rgba),
        );
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(2),
            Stroke::new(
                self.stroke_width.max(0.0) * ctx.scale,
                ranim_to_egui_color(self.stroke_rgba),
            ),
            StrokeKind::Outside,
        );
    }

    fn extract_core_items(&self, _ctx: &RenderCtx, out: &mut Vec<CoreItem>) {
        self.extract_into(out);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl SlideObject for Circle {
    fn descriptor(&self) -> &'static SlideObjectDescriptor {
        &CIRCLE_DESCRIPTOR
    }

    fn bounds(&self) -> Rect {
        let radius = self.radius.abs() as f32;
        let center = pos2(self.center.x as f32, self.center.y as f32);
        Rect::from_center_size(center, vec2(radius * 2.0, radius * 2.0))
    }

    fn bounds3(&self) -> [DVec3; 2] {
        self.aabb()
    }

    fn translate(&mut self, delta: Vec2) {
        self.center.x += delta.x as f64;
        self.center.y += delta.y as f64;
    }

    fn translate3(&mut self, delta: DVec3) {
        self.shift(delta);
    }

    fn set_position(&mut self, pos: Pos2) {
        let size = self.bounds().size();
        self.center.x = (pos.x + size.x / 2.0) as f64;
        self.center.y = (pos.y + size.y / 2.0) as f64;
    }

    fn set_size(&mut self, size: Vec2) {
        let size = clamp_size(size);
        self.radius = (size.x.min(size.y) / 2.0) as f64;
    }

    fn inspector_ui(&mut self, ui: &mut Ui, _ctx: &mut InspectorCtx) -> InspectorResponse {
        let mut changed = false;
        changed |= dvec3_ui(ui, "Center", &mut self.center);
        changed |= ui
            .add(
                DragValue::new(&mut self.radius)
                    .speed(0.01)
                    .range(MIN_OBJECT_SIZE as f64..=8.0)
                    .prefix("Radius "),
            )
            .changed();
        changed |= axes_ui(ui, &mut self.axes);
        changed |= fill_color_ui(ui, &mut self.fill_rgba);
        changed |= stroke_color_ui(ui, &mut self.stroke_rgba);
        changed |= stroke_width_ui(ui, &mut self.stroke_width);
        InspectorResponse::changed(changed)
    }

    fn paint_preview(&self, ui: &Ui, ctx: &PaintCtx) {
        let rect = ctx.scene_rect_to_screen(self.bounds());
        ui.painter().circle_filled(
            rect.center(),
            rect.width().min(rect.height()) / 2.0,
            ranim_to_egui_color(self.fill_rgba),
        );
        ui.painter().circle_stroke(
            rect.center(),
            rect.width().min(rect.height()) / 2.0,
            Stroke::new(
                self.stroke_width.max(0.0) * ctx.scale,
                ranim_to_egui_color(self.stroke_rgba),
            ),
        );
    }

    fn extract_core_items(&self, _ctx: &RenderCtx, out: &mut Vec<CoreItem>) {
        self.extract_into(out);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct SquareObject {
    pub item: Square,
}

#[derive(Debug, Clone)]
pub struct EllipseObject {
    pub item: Ellipse,
}

#[derive(Debug, Clone)]
pub struct LineObject {
    pub item: Line,
}

#[derive(Debug, Clone)]
pub struct ArcObject {
    pub item: Arc,
}

#[derive(Debug, Clone)]
pub struct ArcBetweenPointsObject {
    pub item: ArcBetweenPoints,
}

#[derive(Debug, Clone)]
pub struct EllipticArcObject {
    pub item: EllipticArc,
}

#[derive(Debug, Clone)]
pub struct ParallelogramObject {
    pub item: Parallelogram,
}

#[derive(Debug, Clone)]
pub struct PolygonObject {
    pub item: Polygon,
}

#[derive(Debug, Clone)]
pub struct RegularPolygonObject {
    pub item: RegularPolygon,
}

#[derive(Debug, Clone)]
pub struct SvgObject {
    pub position: DVec3,
    pub scale: DVec3,
    pub intrinsic_bounds: [DVec3; 2],
    pub source: String,
    pub source_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TypstTextObject {
    pub position: DVec3,
    pub scale: DVec3,
    pub intrinsic_bounds: [DVec3; 2],
    pub source: String,
    pub compile_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RawVItemObject {
    pub item: VItem,
}

#[derive(Debug, Clone)]
pub struct SphereObject {
    pub item: Sphere,
}

#[derive(Debug, Clone)]
pub struct SurfaceObject {
    pub item: Surface,
}

#[derive(Debug, Clone)]
pub struct RawMeshObject {
    pub item: MeshItem,
}

trait SlideItemResize {
    fn resize_to(&mut self, bounds: Rect, size: Vec2);
}

impl SlideItemResize for Square {
    fn resize_to(&mut self, bounds: Rect, size: Vec2) {
        let size = clamp_size(size);
        self.size = size.x.min(size.y) as f64;
        self.center = dvec3(
            (bounds.min.x + size.x / 2.0) as f64,
            (bounds.min.y + size.y / 2.0) as f64,
            self.center.z,
        );
    }
}

impl SlideItemResize for Ellipse {
    fn resize_to(&mut self, bounds: Rect, size: Vec2) {
        let size = clamp_size(size);
        self.radius = dvec2(size.x as f64 / 2.0, size.y as f64 / 2.0);
        self.center = dvec3(
            (bounds.min.x + size.x / 2.0) as f64,
            (bounds.min.y + size.y / 2.0) as f64,
            self.center.z,
        );
    }
}

impl SlideItemResize for Line {
    fn resize_to(&mut self, bounds: Rect, size: Vec2) {
        let size = clamp_size(size);
        let [start, end] = self.points;
        self.points = [
            dvec3(bounds.min.x as f64, bounds.min.y as f64, start.z),
            dvec3(
                (bounds.min.x + size.x) as f64,
                (bounds.min.y + size.y) as f64,
                end.z,
            ),
        ];
    }
}

impl SlideItemResize for Arc {
    fn resize_to(&mut self, bounds: Rect, size: Vec2) {
        let size = clamp_size(size);
        self.radius = size.x.min(size.y) as f64 / 2.0;
        self.center = dvec3(
            (bounds.min.x + size.x / 2.0) as f64,
            (bounds.min.y + size.y / 2.0) as f64,
            self.center.z,
        );
    }
}

impl SlideItemResize for ArcBetweenPoints {
    fn resize_to(&mut self, bounds: Rect, size: Vec2) {
        let size = clamp_size(size);
        self.start = dvec3(
            bounds.min.x as f64,
            (bounds.min.y + size.y) as f64,
            self.start.z,
        );
        self.end = dvec3(
            (bounds.min.x + size.x) as f64,
            (bounds.min.y + size.y) as f64,
            self.end.z,
        );
    }
}

impl SlideItemResize for EllipticArc {
    fn resize_to(&mut self, bounds: Rect, size: Vec2) {
        let size = clamp_size(size);
        self.radius = dvec2(size.x as f64 / 2.0, size.y as f64 / 2.0);
        self.center = dvec3(
            (bounds.min.x + size.x / 2.0) as f64,
            (bounds.min.y + size.y / 2.0) as f64,
            self.center.z,
        );
    }
}

impl SlideItemResize for Parallelogram {
    fn resize_to(&mut self, bounds: Rect, size: Vec2) {
        let size = clamp_size(size);
        self.origin = dvec3(bounds.min.x as f64, bounds.min.y as f64, self.origin.z);
        self.axes = (
            dvec3(size.x as f64, 0.0, 0.0),
            dvec3(size.x as f64 * 0.25, size.y as f64, 0.0),
        );
    }
}

impl SlideItemResize for Polygon {
    fn resize_to(&mut self, bounds: Rect, size: Vec2) {
        let size = clamp_size(size);
        let current = clamp_size(bounds.size());
        let sx = size.x / current.x;
        let sy = size.y / current.y;
        for point in &mut self.points {
            point.x = bounds.min.x as f64 + (point.x - bounds.min.x as f64) * sx as f64;
            point.y = bounds.min.y as f64 + (point.y - bounds.min.y as f64) * sy as f64;
        }
    }
}

impl SlideItemResize for RegularPolygon {
    fn resize_to(&mut self, bounds: Rect, size: Vec2) {
        let size = clamp_size(size);
        self.radius = size.x.min(size.y) as f64 / 2.0;
        self.center = dvec3(
            (bounds.min.x + size.x / 2.0) as f64,
            (bounds.min.y + size.y / 2.0) as f64,
            self.center.z,
        );
    }
}

impl SlideItemResize for VItem {
    fn resize_to(&mut self, bounds: Rect, size: Vec2) {
        scale_item_to_size(self, bounds, size);
    }
}

impl SlideItemResize for MeshItem {
    fn resize_to(&mut self, bounds: Rect, size: Vec2) {
        scale_item_to_size(self, bounds, size);
    }
}

trait SlideItemInspector {
    fn inspector_fields_ui(&mut self, ui: &mut Ui) -> bool;
}

impl SlideItemInspector for Square {
    fn inspector_fields_ui(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        changed |= dvec3_ui(ui, "Center", &mut self.center);
        changed |= ui
            .add(
                DragValue::new(&mut self.size)
                    .speed(0.01)
                    .range(MIN_OBJECT_SIZE as f64..=16.0)
                    .prefix("Side "),
            )
            .changed();
        changed |= axes_ui(ui, &mut self.axes);
        changed |= fill_color_ui(ui, &mut self.fill_rgba);
        changed |= stroke_color_ui(ui, &mut self.stroke_rgba);
        changed |= stroke_width_ui(ui, &mut self.stroke_width);
        changed
    }
}

impl SlideItemInspector for Ellipse {
    fn inspector_fields_ui(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        changed |= dvec3_ui(ui, "Center", &mut self.center);
        changed |= dvec2_ui(ui, "Radius", &mut self.radius);
        self.radius.x = self.radius.x.max(MIN_OBJECT_SIZE as f64);
        self.radius.y = self.radius.y.max(MIN_OBJECT_SIZE as f64);
        changed |= axes_ui(ui, &mut self.axes);
        changed |= fill_color_ui(ui, &mut self.fill_rgba);
        changed |= stroke_color_ui(ui, &mut self.stroke_rgba);
        changed |= stroke_width_ui(ui, &mut self.stroke_width);
        changed
    }
}

impl SlideItemInspector for Line {
    fn inspector_fields_ui(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        changed |= dvec3_ui(ui, "Start", &mut self.points[0]);
        changed |= dvec3_ui(ui, "End", &mut self.points[1]);
        ui.label("Extrude");
        ui.horizontal(|ui| {
            changed |= ui
                .add(
                    DragValue::new(&mut self.extrude[0])
                        .speed(0.01)
                        .prefix("A "),
                )
                .changed();
            changed |= ui
                .add(
                    DragValue::new(&mut self.extrude[1])
                        .speed(0.01)
                        .prefix("B "),
                )
                .changed();
        });
        changed |= stroke_color_ui(ui, &mut self.stroke_rgba);
        changed |= stroke_width_ui(ui, &mut self.stroke_width);
        changed
    }
}

impl SlideItemInspector for Arc {
    fn inspector_fields_ui(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        changed |= dvec3_ui(ui, "Center", &mut self.center);
        changed |= ui
            .add(
                DragValue::new(&mut self.radius)
                    .speed(0.01)
                    .range(MIN_OBJECT_SIZE as f64..=16.0)
                    .prefix("Radius "),
            )
            .changed();
        changed |= ui
            .add(DragValue::new(&mut self.angle).speed(0.01).prefix("Angle "))
            .changed();
        changed |= axes_ui(ui, &mut self.axes);
        changed |= stroke_color_ui(ui, &mut self.stroke_rgba);
        changed |= stroke_width_ui(ui, &mut self.stroke_width);
        changed
    }
}

impl SlideItemInspector for ArcBetweenPoints {
    fn inspector_fields_ui(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        changed |= dvec3_ui(ui, "Start", &mut self.start);
        changed |= dvec3_ui(ui, "End", &mut self.end);
        changed |= ui
            .add(DragValue::new(&mut self.angle).speed(0.01).prefix("Angle "))
            .changed();
        changed |= axes_ui(ui, &mut self.axes);
        changed |= stroke_color_ui(ui, &mut self.stroke_rgba);
        changed |= stroke_width_ui(ui, &mut self.stroke_width);
        changed
    }
}

impl SlideItemInspector for EllipticArc {
    fn inspector_fields_ui(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        changed |= dvec3_ui(ui, "Center", &mut self.center);
        changed |= dvec2_ui(ui, "Radius", &mut self.radius);
        self.radius.x = self.radius.x.max(MIN_OBJECT_SIZE as f64);
        self.radius.y = self.radius.y.max(MIN_OBJECT_SIZE as f64);
        changed |= ui
            .add(
                DragValue::new(&mut self.start_angle)
                    .speed(0.01)
                    .prefix("Start "),
            )
            .changed();
        changed |= ui
            .add(DragValue::new(&mut self.angle).speed(0.01).prefix("Angle "))
            .changed();
        changed |= axes_ui(ui, &mut self.axes);
        changed |= stroke_color_ui(ui, &mut self.stroke_rgba);
        changed |= stroke_width_ui(ui, &mut self.stroke_width);
        changed
    }
}

impl SlideItemInspector for Parallelogram {
    fn inspector_fields_ui(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        changed |= dvec3_ui(ui, "Origin", &mut self.origin);
        changed |= axes_ui(ui, &mut self.axes);
        changed |= fill_color_ui(ui, &mut self.fill_rgba);
        changed |= stroke_color_ui(ui, &mut self.stroke_rgba);
        changed |= stroke_width_ui(ui, &mut self.stroke_width);
        changed
    }
}

impl SlideItemInspector for Polygon {
    fn inspector_fields_ui(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        changed |= axes_ui(ui, &mut self.axes);
        changed |= dvec3_points_ui(ui, "Points", &mut self.points, 3, true);
        changed |= fill_color_ui(ui, &mut self.fill_rgba);
        changed |= stroke_color_ui(ui, &mut self.stroke_rgba);
        changed |= stroke_width_ui(ui, &mut self.stroke_width);
        changed
    }
}

impl SlideItemInspector for RegularPolygon {
    fn inspector_fields_ui(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        changed |= dvec3_ui(ui, "Center", &mut self.center);
        changed |= ui
            .add(
                DragValue::new(&mut self.sides)
                    .speed(1.0)
                    .range(3..=64)
                    .prefix("Sides "),
            )
            .changed();
        changed |= ui
            .add(
                DragValue::new(&mut self.radius)
                    .speed(0.01)
                    .range(MIN_OBJECT_SIZE as f64..=16.0)
                    .prefix("Radius "),
            )
            .changed();
        changed |= axes_ui(ui, &mut self.axes);
        changed |= fill_color_ui(ui, &mut self.fill_rgba);
        changed |= stroke_color_ui(ui, &mut self.stroke_rgba);
        changed |= stroke_width_ui(ui, &mut self.stroke_width);
        changed
    }
}

macro_rules! impl_extract_object {
    ($ty:ty, $field:ident, $descriptor:ident) => {
        impl SlideObject for $ty {
            fn descriptor(&self) -> &'static SlideObjectDescriptor {
                &$descriptor
            }

            fn bounds(&self) -> Rect {
                aabb_to_rect(self.$field.aabb())
            }

            fn bounds3(&self) -> [DVec3; 2] {
                self.$field.aabb()
            }

            fn translate(&mut self, delta: Vec2) {
                self.$field
                    .shift(dvec3(delta.x as f64, delta.y as f64, 0.0));
            }

            fn translate3(&mut self, delta: DVec3) {
                self.$field.shift(delta);
            }

            fn set_position(&mut self, pos: Pos2) {
                let bounds = self.bounds();
                self.translate(pos - bounds.min);
            }

            fn set_size(&mut self, size: Vec2) {
                let bounds = self.bounds();
                self.$field.resize_to(bounds, size);
            }

            fn inspector_ui(&mut self, ui: &mut Ui, _ctx: &mut InspectorCtx) -> InspectorResponse {
                let changed = self.$field.inspector_fields_ui(ui);
                InspectorResponse::changed(changed)
            }

            fn paint_preview(&self, _ui: &Ui, _ctx: &PaintCtx) {}

            fn extract_core_items(&self, _ctx: &RenderCtx, out: &mut Vec<CoreItem>) {
                self.$field.extract_into(out);
            }

            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }
    };
}

macro_rules! impl_filled_extract_object {
    ($ty:ty, $field:ident, $descriptor:ident) => {
        impl SlideObject for $ty {
            fn descriptor(&self) -> &'static SlideObjectDescriptor {
                &$descriptor
            }

            fn bounds(&self) -> Rect {
                aabb_to_rect(self.$field.aabb())
            }

            fn bounds3(&self) -> [DVec3; 2] {
                self.$field.aabb()
            }

            fn translate(&mut self, delta: Vec2) {
                self.$field
                    .shift(dvec3(delta.x as f64, delta.y as f64, 0.0));
            }

            fn translate3(&mut self, delta: DVec3) {
                self.$field.shift(delta);
            }

            fn set_position(&mut self, pos: Pos2) {
                let bounds = self.bounds();
                self.translate(pos - bounds.min);
            }

            fn set_size(&mut self, size: Vec2) {
                let bounds = self.bounds();
                self.$field.resize_to(bounds, size);
            }

            fn inspector_ui(&mut self, ui: &mut Ui, _ctx: &mut InspectorCtx) -> InspectorResponse {
                let changed = self.$field.inspector_fields_ui(ui);
                InspectorResponse::changed(changed)
            }

            fn paint_preview(&self, _ui: &Ui, _ctx: &PaintCtx) {}

            fn extract_core_items(&self, _ctx: &RenderCtx, out: &mut Vec<CoreItem>) {
                self.$field.extract_into(out);
            }

            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }
    };
}

impl_filled_extract_object!(SquareObject, item, SQUARE_DESCRIPTOR);
impl_filled_extract_object!(EllipseObject, item, ELLIPSE_DESCRIPTOR);
impl_extract_object!(LineObject, item, LINE_DESCRIPTOR);
impl_extract_object!(ArcObject, item, ARC_DESCRIPTOR);
impl_extract_object!(ArcBetweenPointsObject, item, ARC_BETWEEN_POINTS_DESCRIPTOR);
impl_extract_object!(EllipticArcObject, item, ELLIPTIC_ARC_DESCRIPTOR);
impl_filled_extract_object!(ParallelogramObject, item, PARALLELOGRAM_DESCRIPTOR);
impl_filled_extract_object!(PolygonObject, item, POLYGON_DESCRIPTOR);
impl_filled_extract_object!(RegularPolygonObject, item, REGULAR_POLYGON_DESCRIPTOR);

impl SlideObject for RawVItemObject {
    fn descriptor(&self) -> &'static SlideObjectDescriptor {
        &RAW_VITEM_DESCRIPTOR
    }

    fn bounds(&self) -> Rect {
        aabb_to_rect(self.item.aabb())
    }

    fn bounds3(&self) -> [DVec3; 2] {
        self.item.aabb()
    }

    fn translate(&mut self, delta: Vec2) {
        self.item.shift(dvec3(delta.x as f64, delta.y as f64, 0.0));
    }

    fn translate3(&mut self, delta: DVec3) {
        self.item.shift(delta);
    }

    fn set_position(&mut self, pos: Pos2) {
        let bounds = self.bounds();
        self.translate(pos - bounds.min);
    }

    fn set_size(&mut self, size: Vec2) {
        let bounds = self.bounds();
        self.item.resize_to(bounds, size);
    }

    fn inspector_ui(&mut self, ui: &mut Ui, _ctx: &mut InspectorCtx) -> InspectorResponse {
        let mut changed = false;
        changed |= optional_dvec3_ui(ui, "Normal", &mut self.item.normal, DVec3::Z);
        changed |= dvec3_points_ui(ui, "VPoints", &mut self.item.vpoints.0, 1, false);
        let mut fill = self.item.fill_color();
        let mut stroke = self.item.stroke_color();
        let mut stroke_width = self.item.stroke_width();
        if fill_color_ui(ui, &mut fill) {
            self.item.set_fill_color(fill);
            changed = true;
        }
        if stroke_color_ui(ui, &mut stroke) {
            self.item.set_stroke_color(stroke);
            changed = true;
        }
        if stroke_width_ui(ui, &mut stroke_width) {
            self.item.set_stroke_width(stroke_width);
            changed = true;
        }
        InspectorResponse::changed(changed)
    }

    fn paint_preview(&self, _ui: &Ui, _ctx: &PaintCtx) {}

    fn extract_core_items(&self, _ctx: &RenderCtx, out: &mut Vec<CoreItem>) {
        self.item.extract_into(out);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl SvgObject {
    fn new_default() -> Self {
        let source = r##"<svg viewBox="0 0 120 100" xmlns="http://www.w3.org/2000/svg">
<path d="M60 5 L115 95 H5 Z" fill="#4a7df0" stroke="#202833" stroke-width="6"/>
</svg>"##
            .to_owned();
        let mut object = Self {
            position: dvec3(-1.2, -1.0, 0.0),
            scale: DVec3::ONE,
            intrinsic_bounds: [DVec3::ZERO, dvec3(2.4, 2.0, 0.0)],
            source,
            source_error: None,
        };
        object.refresh_intrinsic_bounds();
        source_object_set_size(&mut object.scale, object.intrinsic_bounds, vec2(2.4, 2.0));
        object
    }

    fn refresh_intrinsic_bounds(&mut self) -> bool {
        match try_svg_item(&self.source) {
            Ok(item) => {
                self.intrinsic_bounds = item.aabb();
                self.source_error = None;
                true
            }
            Err(err) => {
                self.source_error = Some(err);
                false
            }
        }
    }
}

impl SlideObject for SvgObject {
    fn descriptor(&self) -> &'static SlideObjectDescriptor {
        &SVG_DESCRIPTOR
    }

    fn bounds(&self) -> Rect {
        aabb_to_rect(self.bounds3())
    }

    fn bounds3(&self) -> [DVec3; 2] {
        source_object_bounds3(self.position, self.intrinsic_bounds, self.scale)
    }

    fn translate(&mut self, delta: Vec2) {
        self.position += dvec3(delta.x as f64, delta.y as f64, 0.0);
    }

    fn translate3(&mut self, delta: DVec3) {
        self.position += delta;
    }

    fn set_position(&mut self, pos: Pos2) {
        self.position.x = pos.x as f64;
        self.position.y = pos.y as f64;
    }

    fn set_size(&mut self, size: Vec2) {
        source_object_set_size(&mut self.scale, self.intrinsic_bounds, size);
    }

    fn inspector_ui(&mut self, ui: &mut Ui, _ctx: &mut InspectorCtx) -> InspectorResponse {
        let mut changed = false;
        ui.label("SVG source");
        if ui.text_edit_multiline(&mut self.source).changed() {
            changed = true;
            self.refresh_intrinsic_bounds();
        }
        changed |= dvec3_ui(ui, "Scale", &mut self.scale);
        clamp_source_scale(&mut self.scale);
        if let Some(err) = &self.source_error {
            ui.colored_label(Color32::from_rgb(190, 60, 60), err);
        }
        InspectorResponse::changed(changed)
    }

    fn paint_preview(&self, _ui: &Ui, _ctx: &PaintCtx) {}

    fn extract_core_items(&self, _ctx: &RenderCtx, out: &mut Vec<CoreItem>) {
        let Ok(mut item) = try_svg_item(&self.source) else {
            return;
        };
        place_source_item(&mut item, self.position, self.intrinsic_bounds, self.scale);
        item.extract_into(out);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl TypstTextObject {
    fn new_default() -> Self {
        let source = "Typst".to_owned();
        let mut object = Self {
            position: dvec3(-1.2, -0.5, 0.0),
            scale: DVec3::ONE,
            intrinsic_bounds: [DVec3::ZERO, dvec3(2.4, 1.0, 0.0)],
            source,
            compile_error: None,
        };
        object.refresh_intrinsic_bounds();
        source_object_set_size(&mut object.scale, object.intrinsic_bounds, vec2(2.4, 1.0));
        object
    }

    fn refresh_intrinsic_bounds(&mut self) -> bool {
        match TypstText::try_new(&self.source) {
            Ok(item) => {
                self.intrinsic_bounds = item.aabb();
                self.compile_error = None;
                true
            }
            Err(err) => {
                self.compile_error = Some(err);
                false
            }
        }
    }
}

impl SlideObject for TypstTextObject {
    fn descriptor(&self) -> &'static SlideObjectDescriptor {
        &TYPST_TEXT_DESCRIPTOR
    }

    fn bounds(&self) -> Rect {
        aabb_to_rect(self.bounds3())
    }

    fn bounds3(&self) -> [DVec3; 2] {
        source_object_bounds3(self.position, self.intrinsic_bounds, self.scale)
    }

    fn translate(&mut self, delta: Vec2) {
        self.position += dvec3(delta.x as f64, delta.y as f64, 0.0);
    }

    fn translate3(&mut self, delta: DVec3) {
        self.position += delta;
    }

    fn set_position(&mut self, pos: Pos2) {
        self.position.x = pos.x as f64;
        self.position.y = pos.y as f64;
    }

    fn set_size(&mut self, size: Vec2) {
        source_object_set_size(&mut self.scale, self.intrinsic_bounds, size);
    }

    fn inspector_ui(&mut self, ui: &mut Ui, _ctx: &mut InspectorCtx) -> InspectorResponse {
        let mut changed = false;
        ui.label("Typst source");
        if ui.text_edit_multiline(&mut self.source).changed() {
            changed = true;
            self.refresh_intrinsic_bounds();
        }
        changed |= dvec3_ui(ui, "Scale", &mut self.scale);
        clamp_source_scale(&mut self.scale);
        if let Some(err) = &self.compile_error {
            ui.colored_label(Color32::from_rgb(190, 60, 60), err);
        }
        InspectorResponse::changed(changed)
    }

    fn paint_preview(&self, _ui: &Ui, _ctx: &PaintCtx) {}

    fn extract_core_items(&self, _ctx: &RenderCtx, out: &mut Vec<CoreItem>) {
        let Ok(mut item) = TypstText::try_new(&self.source) else {
            return;
        };
        place_source_item(&mut item, self.position, self.intrinsic_bounds, self.scale);
        item.extract_into(out);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl SlideObject for SphereObject {
    fn descriptor(&self) -> &'static SlideObjectDescriptor {
        &SPHERE_DESCRIPTOR
    }

    fn bounds(&self) -> Rect {
        aabb_to_rect(self.item.aabb())
    }

    fn bounds3(&self) -> [DVec3; 2] {
        self.item.aabb()
    }

    fn translate(&mut self, delta: Vec2) {
        self.item.shift(dvec3(delta.x as f64, delta.y as f64, 0.0));
    }

    fn translate3(&mut self, delta: DVec3) {
        self.item.shift(delta);
    }

    fn set_position(&mut self, pos: Pos2) {
        let bounds = self.bounds();
        self.translate(pos - bounds.min);
    }

    fn set_size(&mut self, size: Vec2) {
        self.item.radius = (size.x.min(size.y) / 2.0).max(MIN_OBJECT_SIZE) as f64;
    }

    fn inspector_ui(&mut self, ui: &mut Ui, _ctx: &mut InspectorCtx) -> InspectorResponse {
        let mut changed = false;
        changed |= dvec3_ui(ui, "Center", &mut self.item.center);
        changed |= ui
            .add(
                DragValue::new(&mut self.item.radius)
                    .speed(0.01)
                    .range(MIN_OBJECT_SIZE as f64..=8.0)
                    .prefix("Radius "),
            )
            .changed();
        changed |= resolution_ui(ui, "Resolution", &mut self.item.resolution);
        changed |= fill_color_ui(ui, &mut self.item.fill_rgba);
        InspectorResponse::changed(changed)
    }

    fn paint_preview(&self, _ui: &Ui, _ctx: &PaintCtx) {}

    fn extract_core_items(&self, _ctx: &RenderCtx, out: &mut Vec<CoreItem>) {
        self.item.extract_into(out);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl SlideObject for SurfaceObject {
    fn descriptor(&self) -> &'static SlideObjectDescriptor {
        &SURFACE_DESCRIPTOR
    }

    fn bounds(&self) -> Rect {
        aabb_to_rect(MeshItem::from(self.item.clone()).aabb())
    }

    fn bounds3(&self) -> [DVec3; 2] {
        MeshItem::from(self.item.clone()).aabb()
    }

    fn translate(&mut self, delta: Vec2) {
        self.item.transform = DMat4::from_translation(dvec3(delta.x as f64, delta.y as f64, 0.0))
            * self.item.transform;
    }

    fn translate3(&mut self, delta: DVec3) {
        self.item.transform = DMat4::from_translation(delta) * self.item.transform;
    }

    fn set_position(&mut self, pos: Pos2) {
        let bounds = self.bounds();
        self.translate(pos - bounds.min);
    }

    fn set_size(&mut self, size: Vec2) {
        let size = clamp_size(size);
        let bounds = self.bounds();
        let current = clamp_size(bounds.size());
        self.item.transform = DMat4::from_scale(dvec3(
            (size.x / current.x) as f64,
            (size.y / current.y) as f64,
            1.0,
        )) * self.item.transform;
    }

    fn inspector_ui(&mut self, ui: &mut Ui, _ctx: &mut InspectorCtx) -> InspectorResponse {
        let mut changed = false;
        ui.label(format!(
            "Resolution {} x {}",
            self.item.resolution.0, self.item.resolution.1
        ));
        changed |= dvec3_points_ui(ui, "Vertices", &mut self.item.vertices, 4, false);
        changed |= dvec3_points_ui(ui, "Normals", &mut self.item.vertex_normals, 0, false);
        let mut color = self.item.fill_color();
        if fill_color_ui(ui, &mut color) {
            self.item.set_fill_color(color);
            changed = true;
        }
        InspectorResponse::changed(changed)
    }

    fn paint_preview(&self, _ui: &Ui, _ctx: &PaintCtx) {}

    fn extract_core_items(&self, _ctx: &RenderCtx, out: &mut Vec<CoreItem>) {
        self.item.extract_into(out);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl SlideObject for RawMeshObject {
    fn descriptor(&self) -> &'static SlideObjectDescriptor {
        &RAW_MESH_DESCRIPTOR
    }

    fn bounds(&self) -> Rect {
        aabb_to_rect(self.item.aabb())
    }

    fn bounds3(&self) -> [DVec3; 2] {
        self.item.aabb()
    }

    fn translate(&mut self, delta: Vec2) {
        self.item.shift(dvec3(delta.x as f64, delta.y as f64, 0.0));
    }

    fn translate3(&mut self, delta: DVec3) {
        self.item.shift(delta);
    }

    fn set_position(&mut self, pos: Pos2) {
        let bounds = self.bounds();
        self.translate(pos - bounds.min);
    }

    fn set_size(&mut self, size: Vec2) {
        let bounds = self.bounds();
        self.item.resize_to(bounds, size);
    }

    fn inspector_ui(&mut self, ui: &mut Ui, _ctx: &mut InspectorCtx) -> InspectorResponse {
        let mut changed = false;
        changed |= vec3_points_ui(ui, "Vertices", &mut self.item.points, 1);
        ui.label(format!(
            "Triangles {}",
            self.item.triangle_indices.len() / 3
        ));
        let mut color = self.item.fill_color();
        if fill_color_ui(ui, &mut color) {
            self.item.set_fill_color(color);
            changed = true;
        }
        InspectorResponse::changed(changed)
    }

    fn paint_preview(&self, _ui: &Ui, _ctx: &PaintCtx) {}

    fn extract_core_items(&self, _ctx: &RenderCtx, out: &mut Vec<CoreItem>) {
        self.item.extract_into(out);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

slide_object_descriptor!(
    RECTANGLE_DESCRIPTOR,
    "ranim_items::vitem::geometry::polygon::Rectangle",
    "Rectangle",
    || {
        let mut rect = Rectangle::from_min_size(dvec3(-1.8, -1.0, 0.0), dvec2(3.6, 2.0));
        rect.fill_rgba = egui_to_ranim_color(Color32::from_rgb(68, 119, 245));
        rect.stroke_rgba = egui_to_ranim_color(Color32::from_rgb(245, 247, 250));
        rect.stroke_width = DEFAULT_SHAPE_STROKE_WIDTH;
        Box::new(rect)
    }
);

slide_object_descriptor!(
    TEXT_DESCRIPTOR,
    "ranim_items::vitem::text::TextItem",
    "Text",
    || Box::new(TextObject::new_default())
);

slide_object_descriptor!(
    CIRCLE_DESCRIPTOR,
    "ranim_items::vitem::geometry::circle::Circle",
    "Circle",
    || {
        let mut circle = Circle::new(1.0);
        circle.center = dvec3(0.0, 0.0, 0.0);
        circle.fill_rgba = egui_to_ranim_color(Color32::from_rgb(67, 180, 129));
        circle.stroke_rgba = egui_to_ranim_color(Color32::from_rgb(245, 247, 250));
        circle.stroke_width = DEFAULT_SHAPE_STROKE_WIDTH;
        Box::new(circle)
    }
);

slide_object_descriptor!(
    SQUARE_DESCRIPTOR,
    "ranim_items::vitem::geometry::polygon::Square",
    "Square",
    || {
        let mut item = Square::new(2.0);
        item.center = dvec3(0.0, 0.0, 0.0);
        item.fill_rgba = egui_to_ranim_color(Color32::from_rgb(110, 135, 245));
        item.stroke_rgba = egui_to_ranim_color(Color32::from_rgb(245, 247, 250));
        item.stroke_width = DEFAULT_SHAPE_STROKE_WIDTH;
        Box::new(SquareObject { item })
    }
);

slide_object_descriptor!(
    ELLIPSE_DESCRIPTOR,
    "ranim_items::vitem::geometry::ellipse::Ellipse",
    "Ellipse",
    || {
        let mut item = Ellipse::new(dvec2(1.8, 1.0));
        item.center = dvec3(0.0, 0.0, 0.0);
        item.fill_rgba = egui_to_ranim_color(Color32::from_rgb(67, 180, 129));
        item.stroke_rgba = egui_to_ranim_color(Color32::from_rgb(245, 247, 250));
        item.stroke_width = DEFAULT_SHAPE_STROKE_WIDTH;
        Box::new(EllipseObject { item })
    }
);

slide_object_descriptor!(
    LINE_DESCRIPTOR,
    "ranim_items::vitem::geometry::line::Line",
    "Line",
    || {
        let mut item = Line::new(dvec3(-2.0, 0.0, 0.0), dvec3(2.0, 0.0, 0.0));
        item.stroke_rgba = egui_to_ranim_color(Color32::from_rgb(40, 46, 56));
        item.stroke_width = EMPHASIS_STROKE_WIDTH;
        Box::new(LineObject { item })
    }
);

slide_object_descriptor!(
    ARC_DESCRIPTOR,
    "ranim_items::vitem::geometry::arc::Arc",
    "Arc",
    || {
        let mut item = Arc::new(std::f64::consts::PI * 1.35, 1.35);
        item.center = dvec3(0.0, 0.0, 0.0);
        item.stroke_rgba = egui_to_ranim_color(Color32::from_rgb(220, 100, 80));
        item.stroke_width = EMPHASIS_STROKE_WIDTH;
        Box::new(ArcObject { item })
    }
);

slide_object_descriptor!(
    ARC_BETWEEN_POINTS_DESCRIPTOR,
    "ranim_items::vitem::geometry::arc::ArcBetweenPoints",
    "Arc Between Points",
    || {
        let mut item = ArcBetweenPoints::new(
            dvec3(-1.9, -0.7, 0.0),
            dvec3(1.9, -0.7, 0.0),
            std::f64::consts::PI * 0.65,
        );
        item.stroke_rgba = egui_to_ranim_color(Color32::from_rgb(220, 100, 80));
        item.stroke_width = EMPHASIS_STROKE_WIDTH;
        Box::new(ArcBetweenPointsObject { item })
    }
);

slide_object_descriptor!(
    ELLIPTIC_ARC_DESCRIPTOR,
    "ranim_items::vitem::geometry::elliptic_arc::EllipticArc",
    "Elliptic Arc",
    || {
        let mut item = EllipticArc::new(0.0, std::f64::consts::PI * 1.4, dvec2(1.8, 1.0));
        item.center = dvec3(0.0, 0.0, 0.0);
        item.stroke_rgba = egui_to_ranim_color(Color32::from_rgb(160, 100, 230));
        item.stroke_width = EMPHASIS_STROKE_WIDTH;
        Box::new(EllipticArcObject { item })
    }
);

slide_object_descriptor!(
    PARALLELOGRAM_DESCRIPTOR,
    "ranim_items::vitem::geometry::parallelogram::Parallelogram",
    "Parallelogram",
    || {
        let mut item = Parallelogram::new(
            dvec3(-1.9, -1.0, 0.0),
            (dvec3(3.2, 0.0, 0.0), dvec3(0.8, 2.0, 0.0)),
        );
        item.fill_rgba = egui_to_ranim_color(Color32::from_rgb(245, 180, 70));
        item.stroke_rgba = egui_to_ranim_color(Color32::from_rgb(245, 247, 250));
        item.stroke_width = DEFAULT_SHAPE_STROKE_WIDTH;
        Box::new(ParallelogramObject { item })
    }
);

slide_object_descriptor!(
    POLYGON_DESCRIPTOR,
    "ranim_items::vitem::geometry::polygon::Polygon",
    "Polygon",
    || {
        let mut item = Polygon::new(vec![
            dvec3(0.0, 1.35, 0.0),
            dvec3(2.0, -0.2, 0.0),
            dvec3(0.8, -1.55, 0.0),
            dvec3(-1.6, -0.7, 0.0),
            dvec3(-1.85, 0.9, 0.0),
        ]);
        item.fill_rgba = egui_to_ranim_color(Color32::from_rgb(90, 170, 230));
        item.stroke_rgba = egui_to_ranim_color(Color32::from_rgb(245, 247, 250));
        item.stroke_width = DEFAULT_SHAPE_STROKE_WIDTH;
        Box::new(PolygonObject { item })
    }
);

slide_object_descriptor!(
    REGULAR_POLYGON_DESCRIPTOR,
    "ranim_items::vitem::geometry::polygon::RegularPolygon",
    "Regular Polygon",
    || {
        let mut item = RegularPolygon::new(6, 1.3);
        item.center = dvec3(0.0, 0.0, 0.0);
        item.fill_rgba = egui_to_ranim_color(Color32::from_rgb(100, 190, 170));
        item.stroke_rgba = egui_to_ranim_color(Color32::from_rgb(245, 247, 250));
        item.stroke_width = DEFAULT_SHAPE_STROKE_WIDTH;
        Box::new(RegularPolygonObject { item })
    }
);

slide_object_descriptor!(
    SVG_DESCRIPTOR,
    "ranim_items::vitem::svg::SvgItem",
    "SVG",
    || { Box::new(SvgObject::new_default()) }
);

slide_object_descriptor!(
    TYPST_TEXT_DESCRIPTOR,
    "ranim_items::vitem::typst::TypstText",
    "Typst Text",
    || { Box::new(TypstTextObject::new_default()) }
);

slide_object_descriptor!(
    RAW_VITEM_DESCRIPTOR,
    "ranim_items::vitem::VItem",
    "Raw VItem",
    || {
        let mut item = VItem::from_vpoints(vec![
            dvec3(-1.0, 1.2, 0.0),
            dvec3(1.0, 1.2, 0.0),
            dvec3(1.6, -0.2, 0.0),
            dvec3(0.35, -1.5, 0.0),
            dvec3(-1.4, -0.55, 0.0),
            dvec3(-1.0, 1.2, 0.0),
        ]);
        item.set_fill_color(egui_to_ranim_color(Color32::from_rgb(120, 165, 245)));
        item.set_stroke_color(egui_to_ranim_color(Color32::from_rgb(245, 247, 250)));
        item.set_stroke_width(DEFAULT_SHAPE_STROKE_WIDTH);
        Box::new(RawVItemObject { item })
    }
);

slide_object_descriptor!(
    SPHERE_DESCRIPTOR,
    "ranim_items::mesh::sphere::Sphere",
    "Sphere",
    || {
        let mut item = Sphere::new(1.0)
            .with_center(dvec3(0.0, 0.0, 0.0))
            .with_resolution((41, 21));
        item.fill_rgba = egui_to_ranim_color(Color32::from_rgb(90, 140, 245));
        Box::new(SphereObject { item })
    }
);

slide_object_descriptor!(
    SURFACE_DESCRIPTOR,
    "ranim_items::mesh::surface::Surface",
    "Surface",
    || {
        let mut item = Surface::from_uv_func(
            |u, v| {
                dvec3(
                    -1.6 + u * 3.2,
                    -1.0 + v * 2.0,
                    (u * std::f64::consts::TAU).sin() * (v * std::f64::consts::TAU).cos() * 0.35,
                )
            },
            (0.0, 1.0),
            (0.0, 1.0),
            (25, 17),
        );
        item.set_fill_color(egui_to_ranim_color(Color32::from_rgb(100, 170, 235)));
        Box::new(SurfaceObject { item })
    }
);

slide_object_descriptor!(
    RAW_MESH_DESCRIPTOR,
    "ranim_items::mesh::MeshItem",
    "Raw Mesh",
    || {
        let mut item = MeshItem::from_indexed_vertices(
            vec![
                vec3(-1.2, 1.0, 0.0),
                vec3(1.3, 0.35, 0.0),
                vec3(-0.2, -1.35, 0.0),
            ],
            vec![0, 1, 2],
        );
        item.set_fill_color(egui_to_ranim_color(Color32::from_rgb(230, 130, 100)));
        Box::new(RawMeshObject { item })
    }
);

fn clamp_size(size: Vec2) -> Vec2 {
    vec2(size.x.max(MIN_OBJECT_SIZE), size.y.max(MIN_OBJECT_SIZE))
}

fn ordered_aabb(aabb: [DVec3; 2]) -> [DVec3; 2] {
    [aabb[0].min(aabb[1]), aabb[0].max(aabb[1])]
}

fn aabb_to_rect(aabb: [DVec3; 2]) -> Rect {
    let [min, max] = ordered_aabb(aabb);
    Rect::from_min_max(
        pos2(min.x as f32, min.y as f32),
        pos2(max.x as f32, max.y as f32),
    )
}

fn scale_item_to_size<T: ScaleTransform>(item: &mut T, bounds: Rect, size: Vec2) {
    let size = clamp_size(size);
    let current = clamp_size(bounds.size());
    let scale = dvec3(
        (size.x / current.x) as f64,
        (size.y / current.y) as f64,
        1.0,
    );
    item.scale(scale);
}

fn source_object_bounds3(
    position: DVec3,
    intrinsic_bounds: [DVec3; 2],
    scale: DVec3,
) -> [DVec3; 2] {
    let [min, max] = ordered_aabb(intrinsic_bounds);
    let size = (max - min) * scale.abs();
    [position, position + size]
}

fn source_object_set_size(scale: &mut DVec3, intrinsic_bounds: [DVec3; 2], size: Vec2) {
    let size = clamp_size(size);
    let [min, max] = ordered_aabb(intrinsic_bounds);
    let intrinsic_size = max - min;
    if intrinsic_size.x.abs() > f64::EPSILON {
        scale.x = size.x as f64 / intrinsic_size.x.abs();
    }
    if intrinsic_size.y.abs() > f64::EPSILON {
        scale.y = size.y as f64 / intrinsic_size.y.abs();
    }
    clamp_source_scale(scale);
}

fn place_source_item<T>(item: &mut T, position: DVec3, intrinsic_bounds: [DVec3; 2], scale: DVec3)
where
    T: ScaleTransform + ShiftTransform,
{
    let [min, _] = ordered_aabb(intrinsic_bounds);
    item.scale(scale);
    item.shift(position - min * scale);
}

fn clamp_source_scale(scale: &mut DVec3) {
    scale.x = scale.x.abs().max(0.001);
    scale.y = scale.y.abs().max(0.001);
    scale.z = scale.z.abs().max(0.001);
}

fn try_svg_item(source: &str) -> Result<SvgItem, String> {
    catch_unwind(AssertUnwindSafe(|| SvgItem::new(source)))
        .map_err(|payload| format!("failed to parse SVG: {}", panic_payload_to_string(payload)))
}

fn panic_payload_to_string(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_owned()
    } else {
        "unknown panic".to_owned()
    }
}

fn dvec2_ui(ui: &mut Ui, label: &str, value: &mut DVec2) -> bool {
    let mut changed = false;
    ui.label(label);
    ui.horizontal(|ui| {
        changed |= ui
            .add(DragValue::new(&mut value.x).speed(0.01).prefix("X "))
            .changed();
        changed |= ui
            .add(DragValue::new(&mut value.y).speed(0.01).prefix("Y "))
            .changed();
    });
    changed
}

fn dvec3_ui(ui: &mut Ui, label: &str, value: &mut DVec3) -> bool {
    let mut changed = false;
    ui.label(label);
    changed |= dvec3_components_ui(ui, value);
    changed
}

fn dvec3_components_ui(ui: &mut Ui, value: &mut DVec3) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui
            .add(DragValue::new(&mut value.x).speed(0.01).prefix("X "))
            .changed();
        changed |= ui
            .add(DragValue::new(&mut value.y).speed(0.01).prefix("Y "))
            .changed();
        changed |= ui
            .add(DragValue::new(&mut value.z).speed(0.01).prefix("Z "))
            .changed();
    });
    changed
}

fn vec3_components_ui(ui: &mut Ui, value: &mut Vec3) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui
            .add(DragValue::new(&mut value.x).speed(0.01).prefix("X "))
            .changed();
        changed |= ui
            .add(DragValue::new(&mut value.y).speed(0.01).prefix("Y "))
            .changed();
        changed |= ui
            .add(DragValue::new(&mut value.z).speed(0.01).prefix("Z "))
            .changed();
    });
    changed
}

fn axes_ui(ui: &mut Ui, axes: &mut (DVec3, DVec3)) -> bool {
    let mut changed = false;
    changed |= dvec3_ui(ui, "Axis U", &mut axes.0);
    changed |= dvec3_ui(ui, "Axis V", &mut axes.1);
    changed
}

fn optional_dvec3_ui(ui: &mut Ui, label: &str, value: &mut Option<DVec3>, default: DVec3) -> bool {
    let mut enabled = value.is_some();
    let mut changed = false;
    if ui.checkbox(&mut enabled, label).changed() {
        *value = enabled.then_some(default);
        changed = true;
    }
    if let Some(value) = value.as_mut() {
        changed |= dvec3_components_ui(ui, value);
    }
    changed
}

fn dvec3_points_ui(
    ui: &mut Ui,
    label: &str,
    points: &mut Vec<DVec3>,
    min_len: usize,
    allow_len_edit: bool,
) -> bool {
    const MAX_VISIBLE_POINTS: usize = 32;

    let mut changed = false;
    ui.collapsing(format!("{label} {}", points.len()), |ui| {
        let visible_len = points.len().min(MAX_VISIBLE_POINTS);
        let mut remove_idx = None;
        for idx in 0..visible_len {
            ui.horizontal(|ui| {
                ui.label(format!("#{idx}"));
                changed |= dvec3_components_ui(ui, &mut points[idx]);
                if allow_len_edit && points.len() > min_len && ui.small_button("Delete").clicked() {
                    remove_idx = Some(idx);
                }
            });
        }

        if let Some(idx) = remove_idx {
            points.remove(idx);
            changed = true;
        }

        if allow_len_edit && ui.button("Add Point").clicked() {
            let point = points.last().copied().unwrap_or(DVec3::ZERO);
            points.push(point);
            changed = true;
        }

        if points.len() > visible_len {
            ui.label(format!("{} more", points.len() - visible_len));
        }
    });
    changed
}

fn vec3_points_ui(ui: &mut Ui, label: &str, points: &mut [Vec3], min_len: usize) -> bool {
    const MAX_VISIBLE_POINTS: usize = 32;

    let mut changed = false;
    ui.collapsing(format!("{label} {}", points.len()), |ui| {
        let visible_len = points.len().min(MAX_VISIBLE_POINTS);
        for (idx, point) in points.iter_mut().take(visible_len).enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("#{idx}"));
                changed |= vec3_components_ui(ui, point);
            });
        }

        if points.len() > visible_len {
            ui.label(format!("{} more", points.len() - visible_len));
        }
        if points.len() < min_len {
            ui.label(format!("Needs at least {min_len}"));
        }
    });
    changed
}

fn resolution_ui(ui: &mut Ui, label: &str, resolution: &mut (u32, u32)) -> bool {
    let mut changed = false;
    ui.label(label);
    ui.horizontal(|ui| {
        changed |= ui
            .add(
                DragValue::new(&mut resolution.0)
                    .speed(1.0)
                    .range(2..=256)
                    .prefix("U "),
            )
            .changed();
        changed |= ui
            .add(
                DragValue::new(&mut resolution.1)
                    .speed(1.0)
                    .range(2..=256)
                    .prefix("V "),
            )
            .changed();
    });
    changed
}

fn fill_color_ui(ui: &mut Ui, color: &mut AlphaColor<Srgb>) -> bool {
    let mut egui_color = ranim_to_egui_color(*color);
    ui.label("Fill");
    if ui.color_edit_button_srgba(&mut egui_color).changed() {
        *color = egui_to_ranim_color(egui_color);
        true
    } else {
        false
    }
}

fn stroke_color_ui(ui: &mut Ui, color: &mut AlphaColor<Srgb>) -> bool {
    let mut egui_color = ranim_to_egui_color(*color);
    ui.label("Stroke");
    if ui.color_edit_button_srgba(&mut egui_color).changed() {
        *color = egui_to_ranim_color(egui_color);
        true
    } else {
        false
    }
}

fn stroke_width_ui(ui: &mut Ui, stroke_width: &mut f32) -> bool {
    ui.add(
        DragValue::new(stroke_width)
            .speed(0.005)
            .range(0.0..=1.0)
            .prefix("Stroke W "),
    )
    .changed()
}

fn egui_to_ranim_color(color: Color32) -> AlphaColor<Srgb> {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    AlphaColor::from_rgba8(r, g, b, a)
}

fn ranim_to_egui_color(color: AlphaColor<Srgb>) -> Color32 {
    let [r, g, b, a] = color.components;
    Color32::from_rgba_unmultiplied(
        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (b.clamp(0.0, 1.0) * 255.0).round() as u8,
        (a.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_type_ids_follow_rust_type_paths() {
        assert_eq!(
            RECTANGLE_DESCRIPTOR.type_id,
            std::any::type_name::<Rectangle>()
        );
        assert_eq!(TEXT_DESCRIPTOR.type_id, std::any::type_name::<TextItem>());
        assert!(RECTANGLE_DESCRIPTOR.type_id.contains("::"));
    }

    #[test]
    fn editor_position_tracks_z_for_wrapped_text() {
        let mut text = TextObject::new_default();
        text.set_position3(dvec3(1.0, 2.0, 3.0));

        let position = text.position();
        assert_eq!(position.x, 1.0);
        assert_eq!(position.y, 2.0);
        assert_eq!(position.z, 3.0);
    }

    #[test]
    fn translucent_colors_round_trip_without_premultiply_drift() {
        let original = Color32::from_rgba_unmultiplied(240, 80, 20, 96);

        let ranim_color = egui_to_ranim_color(original);
        assert_color_near(
            ranim_color,
            [240.0 / 255.0, 80.0 / 255.0, 20.0 / 255.0, 96.0 / 255.0],
        );

        let egui_color = ranim_to_egui_color(ranim_color);
        assert_rgba8_near(egui_color.to_srgba_unmultiplied(), [240, 80, 20, 96]);
    }

    #[test]
    fn typst_compile_errors_do_not_panic_the_editor() {
        let mut object = TypstTextObject::new_default();
        let previous_bounds = object.bounds();

        object.source = "#let broken =".to_owned();
        assert!(!object.refresh_intrinsic_bounds());
        assert!(
            object
                .compile_error
                .as_deref()
                .is_some_and(|err| err.contains("Error"))
        );
        assert_eq!(object.bounds(), previous_bounds);

        let mut items = Vec::new();
        object.extract_core_items(&RenderCtx, &mut items);
        assert!(items.is_empty());
    }

    #[test]
    fn typst_bounds_follow_intrinsic_content_width() {
        let mut object = TypstTextObject::new_default();
        let short_width = object.bounds().width();

        object.source = "Typst Typst Typst".to_owned();
        assert!(object.refresh_intrinsic_bounds());
        assert!(object.bounds().width() > short_width * 2.0);
    }

    fn assert_color_near(actual: AlphaColor<Srgb>, expected: [f32; 4]) {
        for (actual, expected) in actual.components.into_iter().zip(expected) {
            assert!(
                (actual - expected).abs() <= 1.0 / 255.0,
                "{actual} != {expected}"
            );
        }
    }

    fn assert_rgba8_near(actual: [u8; 4], expected: [u8; 4]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(actual.abs_diff(expected) <= 1, "{actual} != {expected}");
        }
    }
}
