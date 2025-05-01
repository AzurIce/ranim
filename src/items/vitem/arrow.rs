use glam::DVec3;
use ranim_macros::{
    Alignable, BoundingBox, Empty, Fill, Interpolatable, Opacity, Partial, Position, Stroke,
};

use crate::{
    components::Anchor,
    items::Blueprint,
    render::primitives::{Extract, Primitive, Renderable, RenderableItem, vitem::VItemPrimitive},
    traits::Position,
};

use super::{Polygon, VItem, line::Line};

/// An arrow tip
///
/// the default tip is like:
///
/// ```text
///             + 0.2 * Y
///            / \
///           /   \
/// 0.1 * -X +-----+ 0.1 * X
/// ```
#[derive(
    Clone, Interpolatable, Alignable, Opacity, Empty, Stroke, Fill, BoundingBox, Position, Partial,
)]
pub struct ArrowTip(pub VItem);

impl Default for ArrowTip {
    fn default() -> Self {
        let vitem = Polygon(vec![
            0.2 * DVec3::Y,
            0.1 * DVec3::X,
            0.1 * DVec3::NEG_X,
            0.2 * DVec3::Y,
        ])
        .build();
        Self(vitem)
    }
}

impl ArrowTip {
    pub fn set_direction(&mut self, dir: DVec3) -> &mut Self {
        let current_dir = self.direction().normalize();
        let new_dir = dir.normalize();
        let rotation_angle = current_dir.angle_between(new_dir);
        let mut rotation_axis = current_dir.cross(new_dir).normalize();
        if rotation_axis.length() < f64::EPSILON {
            rotation_axis = DVec3::Z;
        }

        if rotation_angle.abs() > f64::EPSILON {
            self.0.rotate(rotation_angle, rotation_axis);
        }
        self
    }
    pub fn put_tip_on(&mut self, pos: DVec3) -> &mut Self {
        let tip_point = self.tip_point();
        self.0.put_anchor_on(Anchor::Point(tip_point), pos);
        self
    }
    pub fn put_bottom_center_on(&mut self, pos: DVec3) -> &mut Self {
        let tip_point = self.tip_point();
        self.0.put_anchor_on(Anchor::Point(tip_point), pos);
        self
    }
    /// The point on the tip
    pub fn tip_point(&self) -> DVec3 {
        *self.0.get_anchor(0).unwrap()
    }
    /// The point at the center of the bottom edge
    pub fn bottom_center_point(&self) -> DVec3 {
        (*self.0.get_anchor(1).unwrap() + *self.0.get_anchor(2).unwrap()) / 2.0
    }
    /// The direction of the tip
    pub fn direction(&self) -> DVec3 {
        self.tip_point() - self.bottom_center_point()
    }
}

impl Extract for ArrowTip {
    type Primitive = VItemPrimitive;
    fn extract(&self) -> <Self::Primitive as Primitive>::Data {
        self.0.extract()
    }
}

#[derive(Clone, Interpolatable, Alignable, Opacity, Empty, Stroke, Fill, Partial)]
pub struct Arrow {
    pub tip: ArrowTip,
    pub line: Line,
}

impl Default for Arrow {
    fn default() -> Self {
        Self::new(-0.2 * DVec3::Y, DVec3::Y)
    }
}

impl Arrow {
    pub fn new(start: DVec3, end: DVec3) -> Self {
        let mut tip = ArrowTip::default();
        tip.set_direction(end - start);
        tip.put_tip_on(end);
        Self {
            line: Line::new(start, tip.bottom_center_point()),
            tip,
        }
    }
    pub fn start(&self) -> DVec3 {
        self.line.start()
    }
    pub fn end(&self) -> DVec3 {
        self.tip.tip_point()
    }
    pub fn put_end_on(&mut self, pos: DVec3) -> &mut Self {
        self.put_start_and_end_on(self.start(), pos)
    }
    pub fn put_start_on(&mut self, pos: DVec3) -> &mut Self {
        self.put_start_and_end_on(pos, self.end())
    }
    pub fn put_start_and_end_on(&mut self, start: DVec3, end: DVec3) -> &mut Self {
        self.tip.set_direction(end - start);
        self.tip.put_tip_on(end);
        self.line
            .put_start_and_end_on(start, self.tip.bottom_center_point());
        self
    }
}

impl RenderableItem for Arrow {
    fn prepare_for_id(
        &self,
        ctx: &crate::context::WgpuContext,
        render_instances: &mut crate::render::primitives::RenderInstances,
        id: usize,
    ) {
        let tip_data = self.tip.extract();
        let line_data = self.line.extract();
        if let Some((tip_instance, line_instance)) =
            render_instances.get_render_instance_mut::<(VItemPrimitive, VItemPrimitive)>(id)
        {
            tip_instance.update(ctx, &tip_data);
            line_instance.update(ctx, &line_data);
        } else {
            render_instances.insert_render_instance(
                id,
                (
                    VItemPrimitive::init(ctx, &tip_data),
                    VItemPrimitive::init(ctx, &line_data),
                ),
            );
        }
    }
    fn renderable_of_id<'a>(
        &'a self,
        render_instances: &'a crate::render::primitives::RenderInstances,
        id: usize,
    ) -> Option<&'a dyn crate::render::primitives::Renderable> {
        render_instances
            .get_render_instance::<(VItemPrimitive, VItemPrimitive)>(id)
            .map(|instance| instance as &dyn Renderable)
    }
}
