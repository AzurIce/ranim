pub mod vmobject;

use std::ops::{Deref, DerefMut};

use crate::{utils::Id, RanimContext};

/// Blueprints are the data structures that are used to create [`Rabject`]s
pub trait Blueprint<T: Rabject> {
    fn build(self) -> RabjectWithId<T>;
}

pub trait Rabject: 'static + Clone {
    type RenderResource;

    /// Used to initialize the render resource when the rabject is extracted
    fn init_render_resource(ctx: &mut RanimContext, rabject: &Self) -> Self::RenderResource;

    fn update_render_resource(
        ctx: &mut RanimContext,
        rabject: &RabjectWithId<Self>,
        render_resource: &mut Self::RenderResource,
    ) where
        Self: Sized;

    #[allow(unused_variables)]
    fn begin_compute_pass<'a>(
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> Option<wgpu::ComputePass<'a>> {
        None
    }

    #[allow(unused_variables)]
    fn compute<'a>(
        ctx: &mut RanimContext,
        compute_pass: &mut wgpu::ComputePass<'a>,
        render_resource: &Self::RenderResource,
    ) {
    }

    fn begin_render_pass<'a>(
        encoder: &'a mut wgpu::CommandEncoder,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
    ) -> wgpu::RenderPass<'a>;

    fn render<'a>(
        ctx: &mut RanimContext,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_resource: &Self::RenderResource,
    );
}

#[derive(Clone)]
pub struct RabjectWithId<T: Rabject> {
    id: Id,
    rabject: T,
}

impl<T: Rabject> Deref for RabjectWithId<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.rabject
    }
}

impl<T: Rabject> DerefMut for RabjectWithId<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.rabject
    }
}

impl<T: Rabject> From<T> for RabjectWithId<T> {
    fn from(rabject: T) -> Self {
        Self {
            id: Id::new(),
            rabject,
        }
    }
}

impl<T: Rabject> RabjectWithId<T> {
    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn extract(&self, ctx: &mut RanimContext) -> ExtractedRabjectWithId<T> {
        ExtractedRabjectWithId {
            id: self.id,
            render_resource: T::init_render_resource(ctx, &self.rabject),
        }
    }
}

pub struct ExtractedRabjectWithId<T: Rabject> {
    id: Id,
    // rabject: Arc<RwLock<T>>,
    pub(crate) render_resource: T::RenderResource,
}

impl<T: Rabject> ExtractedRabjectWithId<T> {
    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn update_render_resource(&mut self, ctx: &mut RanimContext, rabject: &RabjectWithId<T>) {
        T::update_render_resource(ctx, rabject, &mut self.render_resource);
    }
}

pub trait Interpolatable {
    fn lerp(&self, target: &Self, t: f32) -> Self;
}

impl Interpolatable for f32 {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        self + (target - self) * t
    }
}
