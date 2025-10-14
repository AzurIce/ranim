/// Primitive for vitem
pub mod vitem;

use std::{
    any::{Any, TypeId}, collections::HashMap, sync::Arc
};

use variadics_please::{all_tuples, all_tuples_enumerated};

use crate::utils::WgpuContext;

use super::RenderTextures;

/// The RenderResource encapsules the wgpu resources.
///
/// It has a `Data` type that is used to initialize/update the resource.
pub trait RenderResource {
    /// The type used for [`RenderResource::init`] and [`RenderResource::update`].
    type Data;
    /// Initialize a RenderResource using [`RenderResource::Data`]
    fn init(ctx: &WgpuContext, data: &Self::Data) -> Self;
    /// update a RenderResource using [`RenderResource::Data`]
    fn update(&mut self, ctx: &WgpuContext, data: &Self::Data);
}

/// The RenderCommand encodes the commands.
pub trait RenderCommand {
    /// Encode the compute pass command
    fn encode_compute_pass_command(&self, cpass: &mut wgpu::ComputePass);
    /// Encode the render pass command
    fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass);
    /// Encode the render command
    fn encode_render_command(
        &self,
        ctx: &WgpuContext,
        pipelines: &mut super::PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        #[cfg(feature = "profiling")] profiler: &mut wgpu_profiler::GpuProfiler,
    );
    /// Debug
    fn debug(&self, _ctx: &WgpuContext) {}
}

macro_rules! impl_tuple_render_command {
    ($($T:ident),*) => {
        impl<$($T: RenderCommand,)*> RenderCommand for ($($T,)*) {
            fn encode_compute_pass_command(
                &self,
                cpass: &mut wgpu::ComputePass,
            ) {
                #[allow(non_snake_case, reason = "`all_tuples!()` generates non-snake-case variable names.")]
                let ($($T,)*) = self;
                $($T.encode_compute_pass_command(
                    cpass,
                );)*
            }
            fn encode_render_pass_command(
                &self,
                rpass: &mut wgpu::RenderPass,
            ) {
                #[allow(non_snake_case, reason = "`all_tuples!()` generates non-snake-case variable names.")]
                let ($($T,)*) = self;
                $($T.encode_render_pass_command(
                    rpass,
                );)*
            }
            fn encode_render_command(
                &self,
                ctx: &WgpuContext,
                pipelines: &mut super::PipelinesStorage,
                encoder: &mut wgpu::CommandEncoder,
                uniforms_bind_group: &wgpu::BindGroup,
                render_textures: &RenderTextures,
                #[cfg(feature = "profiling")] profiler: &mut wgpu_profiler::GpuProfiler,
            ) {
                #[allow(non_snake_case, reason = "`all_tuples!()` generates non-snake-case variable names.")]
                let ($($T,)*) = self;
                $($T.encode_render_command(
                    ctx,
                    pipelines,
                    encoder,
                    uniforms_bind_group,
                    render_textures,
                    #[cfg(feature = "profiling")]
                    profiler,
                );)*
            }
            fn debug(&self, ctx: &WgpuContext) {
                #[allow(non_snake_case, reason = "`all_tuples!()` generates non-snake-case variable names.")]
                let ($($T,)*) = self;
                $($T.debug(ctx);)*
            }
        }
    };
}

all_tuples!(impl_tuple_render_command, 1, 16, T);

impl<T: RenderCommand> RenderCommand for Vec<T> {
    fn encode_compute_pass_command(&self, cpass: &mut wgpu::ComputePass) {
        self.iter()
            .for_each(|x| x.encode_compute_pass_command(cpass))
    }
    fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        self.iter()
            .for_each(|x| x.encode_render_pass_command(rpass))
    }
    fn encode_render_command(
        &self,
        ctx: &WgpuContext,
        pipelines: &mut super::PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        #[cfg(feature = "profiling")] profiler: &mut wgpu_profiler::GpuProfiler,
    ) {
        self.iter().for_each(|x| {
            x.encode_render_command(
                ctx,
                pipelines,
                encoder,
                uniforms_bind_group,
                render_textures,
                #[cfg(feature = "profiling")]
                profiler,
            )
        });
    }
    fn debug(&self, _ctx: &WgpuContext) {
        self.iter().for_each(|x| x.debug(_ctx));
    }
}

impl<T: RenderCommand, const N: usize> RenderCommand for [T; N] {
    fn encode_compute_pass_command(&self, cpass: &mut wgpu::ComputePass) {
        self.iter()
            .for_each(|x| x.encode_compute_pass_command(cpass))
    }
    fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        self.iter()
            .for_each(|x| x.encode_render_pass_command(rpass))
    }
    fn encode_render_command(
        &self,
        ctx: &WgpuContext,
        pipelines: &mut super::PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        #[cfg(feature = "profiling")] profiler: &mut wgpu_profiler::GpuProfiler,
    ) {
        self.iter().for_each(|x| {
            x.encode_render_command(
                ctx,
                pipelines,
                encoder,
                uniforms_bind_group,
                render_textures,
                #[cfg(feature = "profiling")]
                profiler,
            )
        });
    }
    fn debug(&self, _ctx: &WgpuContext) {
        self.iter().for_each(|x| x.debug(_ctx));
    }
}

// pub trait Extractor<T> {
//     type Target;
//     fn extract(data: &T) -> Self::Target;
// }

// /// Extract a [`Extract::Target`] from reference.
// pub trait Extract {
//     /// The extraction result
//     type Target;
//     /// Extract a [`Extract::Target`] from reference.
//     fn extract(&self) -> Self::Target;
// }

// impl<E: Extract> Extract for Group<E> {
//     type Target = Vec<E::Target>;
//     fn extract(&self) -> Self::Target {
//         self.iter().map(|x| x.extract()).collect()
//     }
// }

/// The Primitive is the basic renderable object in Ranim.
///
/// The Primitive itself is simply the data of the object.
/// A Primitive has a corresponding [`Primitive::RenderInstance`],
/// which implements [`RenderResource`] and [`RenderCommand`]:
/// - [`RenderResource`]: A trait about init or update itself with [`RenderResource::Data`].
/// - [`RenderCommand`]: A trait about encoding GPU commands.
pub trait Primitive {
    /// The RenderInstance
    type RenderInstance: RenderResource<Data = Self> + RenderCommand;
}

/// A trait for type erasing
#[deprecated]
pub trait Renderable {
    /// Prepare render instance for id
    fn prepare_for_id(&self, ctx: &WgpuContext, render_instances: &mut RenderInstances, id: usize);
}
impl<T: Primitive + 'static> Renderable for T {
    fn prepare_for_id(&self, ctx: &WgpuContext, render_instances: &mut RenderInstances, id: usize) {
        if let Some(instance) = render_instances.get_render_instance_mut::<T::RenderInstance>(id) {
            instance.update(ctx, self);
        } else {
            render_instances.insert_render_instance(id, T::RenderInstance::init(ctx, self));
        }
    }
}
impl<T: Primitive + 'static> Renderable for Vec<T> {
    fn prepare_for_id(&self, ctx: &WgpuContext, render_instances: &mut RenderInstances, id: usize) {
        // info!("prepare for vec");
        if self.is_empty() {
            return;
        }
        if let Some(instance) =
            render_instances.get_render_instance_mut::<Vec<T::RenderInstance>>(id)
        {
            // info!("update");
            if instance.len() != self.len() {
                instance.resize_with(self.len(), || T::RenderInstance::init(ctx, &self[0]));
            }
            instance
                .iter_mut()
                .zip(self.iter())
                .for_each(|(instance, data)| {
                    instance.update(ctx, data);
                });
            // info!("update done");
        } else {
            // info!("insert");
            let instances = self
                .iter()
                .map(|data| T::RenderInstance::init(ctx, data))
                .collect::<Vec<_>>();
            // info!("insert");
            render_instances.insert_render_instance(id, instances);
            // info!("insert done");
        }
    }
}

macro_rules! impl_tuple_renderable {
    ($(($n:tt, $T:ident)),*) => {
        impl<$($T: Primitive + 'static),*> Renderable for ($($T,)*) {
            fn prepare_for_id(&self, ctx: &WgpuContext, render_instances: &mut RenderInstances, id: usize) {
                if let Some(instance) =
                    render_instances.get_render_instance_mut::<($($T::RenderInstance,)*)>(id)
                {
                    $(instance.$n.update(ctx, &self.$n);)*
                } else {
                    let instance = (
                        $($T::RenderInstance::init(ctx, &self.$n),)*
                    );
                    render_instances.insert_render_instance(id, instance);
                }
            }
        }
    };
}

all_tuples_enumerated!(impl_tuple_renderable, 1, 16, T);

slotmap::new_key_type! { pub struct RenderInstanceKey; }

pub struct RenderPool {
    inner: slotmap::SlotMap<
        RenderInstanceKey,
        (Arc<RenderInstanceKey>, TypeId, Box<dyn AnyRenderCommand>),
    >,
    last_frame_dropped: HashMap<TypeId, Vec<RenderInstanceKey>>,
}

impl RenderPool {
    pub fn new() -> Self {
        Self {
            inner: slotmap::SlotMap::with_key(),
            last_frame_dropped: HashMap::new(),
        }
    }

    pub fn get(&self, key: RenderInstanceKey) -> Option<&dyn AnyRenderCommand> {
        self.inner.get(key).map(|x| x.2.as_ref() as &dyn AnyRenderCommand)
    }
    
    pub fn show(&self) {
        self.inner.iter().enumerate().for_each(|(idx, (_, (k, _, _)))| {
            print!("{idx}: {}, ", Arc::strong_count(k));
        });
        println!("");
    }

    pub fn alloc<T: RenderCommand + RenderResource<Data = D> + 'static, D>(
        &mut self,
        ctx: &WgpuContext,
        data: &D,
    ) -> Arc<RenderInstanceKey> {
        let last_frame_dropped = self
            .last_frame_dropped
            .entry(TypeId::of::<T>())
            .or_insert(Vec::new());
        if let Some(key) = last_frame_dropped.pop() {
            let entry = self.inner.get_mut(key).unwrap();
            let key = entry.0.clone();
            (entry.2.as_mut() as &mut dyn Any)
                .downcast_mut::<T>()
                .unwrap()
                .update(ctx, data);
            key
        } else {
            let handle = self.inner.insert_with_key(|key| {
                (
                    Arc::new(key),
                    TypeId::of::<T>(),
                    Box::new(T::init(ctx, data)),
                )
            });
            self.inner.get(handle).unwrap().0.clone()
        }
    }

    pub fn clean(&mut self) {
        self.inner.retain(|key, (_, t_id, _)| {
            self.last_frame_dropped
                .get(t_id)
                .map(|x| !x.contains(&key))
                .unwrap_or(true)
        });
        // println!("dropped {}", self.last_frame_dropped.len());
        self.last_frame_dropped.clear();
        self.inner
            .iter()
            .filter(|(_, (key, _, _))| Arc::strong_count(key) == 1)
            .for_each(|(key, (_, t_id, _))| {
                self.last_frame_dropped
                    .entry(*t_id)
                    .or_insert(Vec::new())
                    .push(key);
            });
    }
    // pub fn new_with_max_mem(max_size: u64) -> Self {
    //     Self {
    //         max_size,
    //         size: 0,
    //     }
    // }
}

/// Type erased [`RenderCommand`]
pub trait AnyRenderCommand: Any + RenderCommand {}
impl<T: RenderCommand + Any> AnyRenderCommand for T {}

#[deprecated = "use [`RenderPool`] instead"]
/// Storage for [`RenderCommand`]s
#[derive(Default)]
pub struct RenderInstances {
    // Rabject's id -> RenderInstance
    items: HashMap<usize, Box<dyn AnyRenderCommand>>,
}

impl RenderInstances {
    // pub(crate) fn get_render_instance<T: RenderCommand + 'static>(&self, id: usize) -> Option<&T> {
    //     self.items
    //         .get(&id)
    //         .and_then(|x| (x.as_ref() as &dyn Any).downcast_ref::<T>())
    // }
    pub fn get_render_instance_dyn(&self, id: usize) -> Option<&dyn RenderCommand> {
        self.items
            .get(&id)
            .map(|x| x.as_ref() as &dyn RenderCommand)
    }
    pub fn get_render_instance_mut<T: RenderCommand + 'static>(
        &mut self,
        id: usize,
    ) -> Option<&mut T> {
        // println!("get_render_instance_mut");
        self.items
            .get_mut(&id)
            .and_then(|x| (x.as_mut() as &mut dyn Any).downcast_mut::<T>())
    }
    pub fn insert_render_instance<T: RenderCommand + 'static>(&mut self, id: usize, instance: T) {
        self.items.insert(id, Box::new(instance));
    }
}

impl RenderCommand for Vec<&dyn RenderCommand> {
    fn encode_compute_pass_command(&self, cpass: &mut wgpu::ComputePass) {
        for render_instance in self {
            render_instance.encode_compute_pass_command(cpass);
        }
    }
    fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        for render_instance in self {
            render_instance.encode_render_pass_command(rpass);
        }
    }

    fn encode_render_command(
        &self,
        ctx: &WgpuContext,
        pipelines: &mut super::PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        #[cfg(feature = "profiling")] profiler: &mut wgpu_profiler::GpuProfiler,
    ) {
        for render_instance in self {
            render_instance.encode_render_command(
                ctx,
                pipelines,
                encoder,
                uniforms_bind_group,
                render_textures,
                #[cfg(feature = "profiling")]
                profiler,
            );
        }
    }
    fn debug(&self, ctx: &WgpuContext) {
        for render_instance in self {
            render_instance.debug(ctx);
        }
    }
}
