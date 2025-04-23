use glam::DVec3;
use ranim_macros::{
    Alignable, BoundingBox, Empty, Fill, Interpolatable, Opacity, Position, Stroke,
};

use crate::{
    items::Blueprint,
    render::primitives::{Extract, Primitive, Renderable, RenderableItem, vitem::VItemPrimitive},
};

use super::{Line, Polygon, VItem};

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
#[derive(Clone, Interpolatable, Alignable, Opacity, Empty, Stroke, Fill, BoundingBox, Position)]
pub struct ArrowTip(pub VItem);

impl Default for ArrowTip {
    fn default() -> Self {
        Self(
            Polygon(vec![
                0.2 * DVec3::Y,
                0.1 * DVec3::X,
                0.2 * DVec3::Y,
                0.1 * DVec3::NEG_X,
            ])
            .build(),
        )
    }
}

impl Extract for ArrowTip {
    type Primitive = VItemPrimitive;
    fn extract(&self) -> <Self::Primitive as Primitive>::Data {
        self.0.extract()
    }
}

#[derive(Clone, Interpolatable, Alignable, Opacity, Empty, Stroke, Fill)]
pub struct Arrow {
    pub tip: ArrowTip,
    pub line: VItem,
}

impl Default for Arrow {
    fn default() -> Self {
        Self::new()
    }
}

impl Arrow {
    pub fn new() -> Self {
        Self {
            tip: ArrowTip::default(),
            line: Line(0.2 * DVec3::NEG_Y, 0.2 * DVec3::Y).build(),
        }
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
