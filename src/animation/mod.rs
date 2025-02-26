pub mod composition;
pub mod creation;
pub mod fading;
pub mod freeze;
pub mod transform;

use std::{cell::RefCell, rc::Rc};

use crate::{
    context::WgpuContext,
    items::{Entity, Rabject},
    render::{
        primitives::{ExtractFrom, RenderInstance, RenderInstances},
        DynamicRenderable, RenderTextures, Renderable, StaticRenderable,
    },
    utils::{rate_functions::linear, Id, PipelinesStorage},
};

#[allow(unused)]
use log::trace;

#[derive(Clone)]
pub enum Animation {
    Dynamic(Rc<RefCell<Box<dyn DynamicRenderable>>>),
    Static(Rc<Box<dyn StaticRenderable>>),
}

impl Renderable for Animation {
    fn render(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        pipelines: &mut PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
    ) {
        match self {
            Animation::Dynamic(anim) => {
                anim.render(
                    ctx,
                    render_instances,
                    pipelines,
                    encoder,
                    uniforms_bind_group,
                    render_textures,
                );
            }
            Animation::Static(anim) => {
                anim.render(
                    ctx,
                    render_instances,
                    pipelines,
                    encoder,
                    uniforms_bind_group,
                    render_textures,
                );
            }
        }
    }
}

impl Renderable for Rc<RefCell<Box<dyn DynamicRenderable>>> {
    fn render(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        pipelines: &mut PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
    ) {
        self.borrow().render(
            ctx,
            render_instances,
            pipelines,
            encoder,
            uniforms_bind_group,
            render_textures,
        );
    }
}

impl Renderable for Rc<Box<dyn StaticRenderable>> {
    fn render(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        pipelines: &mut PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
    ) {
        self.as_ref().render(
            ctx,
            render_instances,
            pipelines,
            encoder,
            uniforms_bind_group,
            render_textures,
        );
    }
}

impl DynamicRenderable for Rc<RefCell<Box<dyn DynamicRenderable>>> {
    fn prepare_alpha(
        &mut self,
        alpha: f32,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
    ) {
        self.borrow_mut()
            .prepare_alpha(alpha, ctx, render_instances);
    }
}

impl StaticRenderable for Rc<Box<dyn StaticRenderable>> {
    fn prepare(&self, ctx: &WgpuContext, render_instances: &mut RenderInstances) {
        self.as_ref().prepare(ctx, render_instances);
    }
}

// /// An `Anim` is a box of [`Animator`]
// pub type Anim = Rc<RefCell<Box<dyn Animator>>>;
// /// An `StaticAnim` is a box of [`Renderable`] inside a `Rc`
// ///
// /// This implements [`Animator`] but does nothing on `update_alpha`
// pub type StaticAnim = Rc<Box<dyn Renderable>>;

/// An animator that animates an entity
pub trait PureEvaluator<T: Entity>: Send + Sync {
    fn eval_alpha(&self, alpha: f32) -> T;
}

impl<T: Entity> PureEvaluator<T> for T {
    fn eval_alpha(&self, _alpha: f32) -> T {
        self.clone()
    }
}

// MARK: AnimSchedule

pub struct AnimSchedule<'r, 't, T: Entity, A> {
    pub(crate) rabject: &'r mut Rabject<'t, T>,
    pub(crate) anim: AnimWithParams<A>,
}

impl<'r, 't, T: Entity, A: Freezable<T>> Freezable<T> for AnimSchedule<'r, 't, T, A> {
    fn get_end_freeze_anim(&self) -> StaticEntityAnim<T> {
        self.anim.inner.get_end_freeze_anim()
    }
}

impl<'r, 't, T: Entity + 'static, A> AnimSchedule<'r, 't, T, A> {
    pub fn new(rabject: &'r mut Rabject<'t, T>, anim: impl Into<A>) -> Self {
        Self {
            rabject,
            anim: AnimWithParams::new(anim.into()),
        }
    }
    pub fn with_duration(mut self, duration_secs: f32) -> Self {
        self.anim.params.duration_secs = duration_secs;
        self
    }
    pub fn with_rate_func(mut self, rate_func: fn(f32) -> f32) -> Self {
        self.anim.params.rate_func = rate_func;
        self
    }
}

impl<T: Entity + 'static> AnimSchedule<'_, '_, T, EntityAnim<T>> {
    pub fn apply(self) -> Self {
        if let EntityAnim::Dynamic(anim) = &self.anim.inner {
            self.rabject.data = anim.evaluator.eval_alpha(1.0);
        }
        self
    }
}

impl<T: Entity + 'static> From<AnimSchedule<'_, '_, T, StaticEntityAnim<T>>> for Animation {
    fn from(value: AnimSchedule<'_, '_, T, StaticEntityAnim<T>>) -> Self {
        Self::Static(Rc::new(Box::new(value.anim)))
    }
}
impl<T: Entity + 'static> From<AnimSchedule<'_, '_, T, DynamicEntityAnim<T>>> for Animation {
    fn from(value: AnimSchedule<'_, '_, T, DynamicEntityAnim<T>>) -> Self {
        Self::Dynamic(Rc::new(RefCell::new(Box::new(value.anim))))
    }
}

// MARK: StaticEntityAnim

#[derive(Clone)]
pub struct StaticEntityAnim<T: Entity> {
    id: Id,
    data: T,
}

impl<T: Entity> StaticEntityAnim<T> {
    pub fn new(id: Id, data: T) -> Self {
        Self { id, data }
    }
}
impl<T: Entity> Freezable<T> for StaticEntityAnim<T> {
    fn get_end_freeze_anim(&self) -> StaticEntityAnim<T> {
        self.clone()
    }
}

impl<T: Entity + 'static> StaticRenderable for StaticEntityAnim<T> {
    fn prepare(&self, ctx: &WgpuContext, render_instances: &mut RenderInstances) {
        let render_resource = render_instances.get_or_init::<T>(self.id);
        render_resource.update_from(ctx, &self.data);
    }
}

impl<T: Entity + 'static> Renderable for StaticEntityAnim<T> {
    fn render(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        pipelines: &mut PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
    ) {
        let render_instance = render_instances.get_or_init::<T>(self.id);
        render_instance.encode_render_command(
            ctx,
            pipelines,
            encoder,
            uniforms_bind_group,
            render_textures,
        );
    }
}

// MARK: Freeze

pub trait Freezable<T: Entity> {
    fn get_end_freeze_anim(&self) -> StaticEntityAnim<T>;
}

// MARK: EntityAnim

pub enum EntityAnim<T: Entity> {
    Dynamic(DynamicEntityAnim<T>),
    Static(StaticEntityAnim<T>),
}

impl<T: Entity> From<DynamicEntityAnim<T>> for EntityAnim<T> {
    fn from(value: DynamicEntityAnim<T>) -> Self {
        Self::Dynamic(value)
    }
}

impl<T: Entity> From<StaticEntityAnim<T>> for EntityAnim<T> {
    fn from(value: StaticEntityAnim<T>) -> Self {
        Self::Static(value)
    }
}

impl<T: Entity + 'static> From<EntityAnim<T>> for Animation {
    fn from(value: EntityAnim<T>) -> Self {
        match value {
            EntityAnim::Dynamic(anim) => Animation::Dynamic(Rc::new(RefCell::new(Box::new(anim)))),
            EntityAnim::Static(anim) => Animation::Static(Rc::new(Box::new(anim))),
        }
    }
}

impl<T: Entity> Freezable<T> for EntityAnim<T> {
    fn get_end_freeze_anim(&self) -> StaticEntityAnim<T> {
        match self {
            Self::Dynamic(anim) => anim.get_end_freeze_anim(),
            Self::Static(anim) => anim.get_end_freeze_anim(),
        }
    }
}

// MARK: DynamicEntityAnim

#[derive(Clone)]
pub struct DynamicEntityAnim<T: Entity> {
    id: Id,
    evaluator: Rc<Box<dyn PureEvaluator<T>>>,
}

impl<T: Entity> DynamicEntityAnim<T> {
    pub fn new(id: Id, func: impl PureEvaluator<T> + 'static) -> Self {
        Self {
            id,
            evaluator: Rc::new(Box::new(func)),
        }
    }
}

impl<T: Entity> Freezable<T> for DynamicEntityAnim<T> {
    fn get_end_freeze_anim(&self) -> StaticEntityAnim<T> {
        StaticEntityAnim {
            id: self.id,
            data: self.evaluator.eval_alpha(1.0),
        }
    }
}

impl<T: Entity + 'static> DynamicRenderable for DynamicEntityAnim<T> {
    fn prepare_alpha(
        &mut self,
        alpha: f32,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
    ) {
        let data = self.evaluator.eval_alpha(alpha);
        let render_instance = render_instances.get_or_init::<T>(self.id);
        render_instance.update_from(ctx, &data);
    }
}

impl<T: Entity + 'static> Renderable for DynamicEntityAnim<T> {
    fn render(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        pipelines: &mut PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
    ) {
        let render_instance = render_instances.get_or_init::<T>(self.id);
        render_instance.encode_render_command(
            ctx,
            pipelines,
            encoder,
            uniforms_bind_group,
            render_textures,
        );
    }
}

// MARK: AnimParams

/// The param of an animation
#[derive(Debug, Clone)]
pub struct AnimParams {
    /// Default: 1.0
    pub duration_secs: f32,
    /// Default: linear
    pub rate_func: fn(f32) -> f32,
}

impl Default for AnimParams {
    fn default() -> Self {
        Self {
            duration_secs: 1.0,
            rate_func: linear,
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
pub struct AnimWithParams<A> {
    pub(crate) inner: A,
    pub(crate) params: AnimParams,
}

impl<A> AnimWithParams<A> {
    pub fn new(anim: A) -> Self {
        Self {
            inner: anim,
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

impl<A: Into<Animation>> From<AnimWithParams<A>> for Animation {
    fn from(value: AnimWithParams<A>) -> Self {
        let anim: Animation = value.inner.into();
        match anim {
            Animation::Dynamic(anim) => {
                Animation::Dynamic(Rc::new(RefCell::new(Box::new(AnimWithParams {
                    inner: anim,
                    params: value.params,
                }))))
            }
            Animation::Static(anim) => Animation::Static(Rc::new(Box::new(AnimWithParams {
                inner: anim,
                params: value.params,
            }))),
        }
    }
}

impl<A: DynamicRenderable> DynamicRenderable for AnimWithParams<A> {
    fn prepare_alpha(
        &mut self,
        alpha: f32,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
    ) {
        // trace!("alpha: {alpha}");
        let alpha = (self.params.rate_func)(alpha);
        // trace!("rate_func alpha: {alpha}");
        self.inner.prepare_alpha(alpha, ctx, render_instances);
    }
}

impl<A: StaticRenderable> StaticRenderable for AnimWithParams<A> {
    fn prepare(&self, ctx: &WgpuContext, render_instances: &mut RenderInstances) {
        self.inner.prepare(ctx, render_instances);
    }
}

impl<A: Renderable> Renderable for AnimWithParams<A> {
    fn render(
        &self,
        ctx: &WgpuContext,
        render_instances: &mut RenderInstances,
        pipelines: &mut PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
    ) {
        self.inner.render(
            ctx,
            render_instances,
            pipelines,
            encoder,
            uniforms_bind_group,
            render_textures,
        );
    }
}
