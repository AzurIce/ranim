use glam::dvec3;

use crate::{
    items::Blueprint,
    render::primitives::{Extract, Primitive, Renderable, RenderableItem, vitem::VItemPrimitive},
};

use super::{Circle, Line, VItem};

#[derive(Clone)]
pub struct Arrow {
    pub tip: VItem,
    pub line: VItem,
}

impl Arrow {
    pub fn new() -> Self {
        Self {
            tip: Circle(1.0).build(),
            line: Line(dvec3(0.0, 0.0, 0.0), dvec3(1.0, 0.0, 0.0)).build(),
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
