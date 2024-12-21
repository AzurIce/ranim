#[deprecated(note = "use rabject2d instead")]
pub mod vmobject;
#[deprecated(note = "use rabject2d instead")]
pub mod vpath;

use std::ops::{Deref, DerefMut};

use crate::{context::RanimContext, scene::{entity::Entity, world::camera::Camera}, updater::Updater, utils::Id};

use super::{Primitive, Rabject};

impl<R: Rabject> From<R> for RabjectEntity3d<R> {
    fn from(rabject: R) -> Self {
        Self {
            rabject,
            updaters: vec![],
            render_data: None,
            render_resource: None,
        }
    }
}

/// An rabject entity in the scene, rendered with [`Camera`]
pub struct RabjectEntity3d<R: Rabject> {
    /// The rabject
    pub(crate) rabject: R,
    /// The updaters of this rabject
    pub(crate) updaters: Vec<(Id, Box<dyn Updater<R>>)>,
    /// The extracted data from the rabject
    pub(crate) render_data: Option<R::RenderData>,
    /// The prepared render resource of the rabject
    pub(crate) render_resource: Option<R::RenderResource>,
}

impl<R: Rabject> Deref for RabjectEntity3d<R> {
    type Target = R;
    fn deref(&self) -> &Self::Target {
        &self.rabject
    }
}

impl<R: Rabject> DerefMut for RabjectEntity3d<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.rabject
    }
}

impl<R: Rabject> RabjectEntity3d<R> {
    pub fn insert_updater(&mut self, mut updater: impl Updater<R> + 'static) -> Id {
        let id = Id::new();
        updater.on_create(self);
        self.updaters.push((id, Box::new(updater)));
        id
    }
    pub fn remove_updater(&mut self, id: Id) {
        self.updaters.retain(|(eid, _)| *eid != id);
    }
}

impl<R: Rabject + 'static> Entity for RabjectEntity3d<R> {
    type Renderer = Camera;

    fn tick(&mut self, dt: f32) {
        let rabject = &mut self.rabject;
        self.updaters.retain_mut(|(_, updater)| {
            let keep = updater.on_update(rabject, dt);
            if !keep {
                updater.on_destroy(rabject);
            }
            keep
        });
    }
    fn extract(&mut self) {
        self.render_data = Some(self.rabject.extract());
    }
    fn prepare(&mut self, ctx: &RanimContext) {
        let wgpu_ctx = ctx.wgpu_ctx();
        if let Some(render_resource) = self.render_resource.as_mut() {
            render_resource.update(&wgpu_ctx, self.render_data.as_ref().unwrap());
        } else {
            self.render_resource = Some(R::RenderResource::init(
                &wgpu_ctx,
                self.render_data.as_ref().unwrap(),
            ));
        }
    }
    fn render(&mut self, ctx: &mut RanimContext, renderer: &mut Self::Renderer) {
        let wgpu_ctx = ctx.wgpu_ctx();
        let pipelines = &mut ctx.pipelines;

        let multisample_view = &renderer.multisample_view;
        let target_view = &renderer.render_view;
        let depth_stencil_view = &renderer.depth_stencil_view;
        let uniforms_bind_group = &renderer.uniforms_bind_group.bind_group;

        if let Some(render_resource) = self.render_resource.as_ref() {
            render_resource.render(
                &wgpu_ctx,
                pipelines,
                multisample_view,
                target_view,
                depth_stencil_view,
                uniforms_bind_group,
            );
        }
    }
}