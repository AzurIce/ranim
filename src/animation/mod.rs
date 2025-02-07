pub mod entity;
pub mod timeline;

use std::rc::Rc;

use crate::{
    context::WgpuContext,
    render::{primitives::RenderInstances, CameraFrame, RenderTextures, Renderable},
    utils::{rate_functions::smooth, PipelinesStorage},
};

#[allow(unused)]
use log::trace;

/// An `Animator` is basically an [`Renderable`] which can responds progress alpha change
pub trait Animator: Renderable {
    fn update_alpha(&mut self, alpha: f32);
}

/// An `Anim` is a box of [`Animator`]
pub type Anim = Box<dyn Animator>;
/// An `StaticAnim` is a box of [`Renderable`] inside a `Rc`
///
/// This implements [`Animator`] but does nothing on `update_alpha`
pub type StaticAnim = Rc<Box<dyn Renderable>>;

impl Renderable for StaticAnim {
    fn render(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        pipelines: &mut PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        camera: &CameraFrame,
    ) {
        self.as_ref().render(
            ctx,
            render_instances,
            pipelines,
            encoder,
            uniforms_bind_group,
            render_textures,
            camera,
        );
    }
}
impl Animator for StaticAnim {
    fn update_alpha(&mut self, _alpha: f32) {
        // DO NOTHING
    }
}

/// The param of an animation
#[derive(Debug, Clone)]
pub struct AnimParams {
    /// Default: 1.0
    pub duration_secs: f32,
    /// Default: smooth
    pub rate_func: fn(f32) -> f32,
}

impl Default for AnimParams {
    fn default() -> Self {
        Self {
            duration_secs: 1.0,
            rate_func: smooth,
        }
    }
}

impl AnimParams {
    pub fn with_duration(mut self, duration_secs: f32) -> Self {
        self.duration_secs = duration_secs;
        self
    }
    pub fn with_rate_func(mut self, rate_func: fn(f32) -> f32) -> Self {
        self.rate_func = rate_func;
        self
    }
}

/// An [`Animator`] with [`AnimParams`]
///
/// This is also an [`Animator`]
pub struct AnimWithParams<T: Animator> {
    pub(crate) anim: T,
    pub(crate) params: AnimParams,
}

impl<T: Animator> AnimWithParams<T> {
    pub fn new(anim: T) -> Self {
        Self {
            anim,
            params: AnimParams::default(),
        }
    }
    pub fn with_duration(mut self, secs: f32) -> Self {
        self.params.duration_secs = secs;
        self
    }
    pub fn with_rate_func(mut self, rate_func: fn(f32) -> f32) -> Self {
        self.params.rate_func = rate_func;
        self
    }
}

impl<T: Animator> Renderable for AnimWithParams<T> {
    fn render(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        pipelines: &mut PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        camera: &CameraFrame,
    ) {
        self.anim.render(
            ctx,
            render_instances,
            pipelines,
            encoder,
            uniforms_bind_group,
            render_textures,
            camera,
        );
    }
}

impl<T: Animator> Animator for AnimWithParams<T> {
    fn update_alpha(&mut self, alpha: f32) {
        // trace!("alpha: {alpha}");
        let alpha = (self.params.rate_func)(alpha);
        // trace!("rate_func alpha: {alpha}");
        self.anim.update_alpha(alpha);
    }
}
