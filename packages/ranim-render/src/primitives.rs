/// Primitive for vitem
pub mod vitem;
pub mod vitem2d;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use variadics_please::all_tuples;

use crate::utils::WgpuContext;

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
    fn encode_depth_render_pass_command(&self, rpass: &mut wgpu::RenderPass);
    /// Encode the render pass command
    fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass);

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
            fn encode_depth_render_pass_command(
                &self,
                rpass: &mut wgpu::RenderPass,
            ) {
                #[allow(non_snake_case, reason = "`all_tuples!()` generates non-snake-case variable names.")]
                let ($($T,)*) = self;
                $($T.encode_depth_render_pass_command(
                    rpass,
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
    fn encode_depth_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        self.iter()
            .for_each(|x| x.encode_depth_render_pass_command(rpass))
    }
    fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        self.iter()
            .for_each(|x| x.encode_render_pass_command(rpass))
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
    fn encode_depth_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        self.iter()
            .for_each(|x| x.encode_depth_render_pass_command(rpass))
    }
    fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        self.iter()
            .for_each(|x| x.encode_render_pass_command(rpass))
    }
    fn debug(&self, _ctx: &WgpuContext) {
        self.iter().for_each(|x| x.debug(_ctx));
    }
}

/// The Primitive is the basic renderable object in Ranim.
///
/// The Primitive itself is simply the data of the object.
/// A Primitive has a corresponding [`Primitive::RenderInstance`],
/// which implements [`RenderResource`] and [`RenderCommand`]:
/// - [`RenderResource`]: A trait about init or update itself with [`RenderResource::Data`].
/// - [`RenderCommand`]: A trait about encoding GPU commands.
pub trait Primitive {
    /// The RenderInstance
    type RenderInstance: RenderResource<Data = Self> + RenderCommand + Send + Sync + 'static;
}

slotmap::new_key_type! { pub struct RenderInstanceKey; }

// MARK: RenderPool
#[derive(Default)]
pub struct RenderPool {
    #[allow(clippy::type_complexity)]
    inner: slotmap::SlotMap<
        RenderInstanceKey,
        (Arc<RenderInstanceKey>, TypeId, Box<dyn AnyRenderCommand>),
    >,
    last_frame_dropped: HashMap<TypeId, Vec<RenderInstanceKey>>,
}

impl RenderPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: RenderInstanceKey) -> Option<&dyn AnyRenderCommand> {
        self.inner
            .get(key)
            .map(|x| x.2.as_ref() as &dyn AnyRenderCommand)
    }

    pub fn show(&self) {
        self.inner
            .iter()
            .enumerate()
            .for_each(|(idx, (_, (k, _, _)))| {
                print!("{idx}: {}, ", Arc::strong_count(k));
            });
        println!();
    }

    pub fn alloc<P: Primitive>(&mut self, ctx: &WgpuContext, data: &P) -> Arc<RenderInstanceKey> {
        let last_frame_dropped = self
            .last_frame_dropped
            .entry(TypeId::of::<P::RenderInstance>())
            .or_default();
        if let Some(key) = last_frame_dropped.pop() {
            let entry = self.inner.get_mut(key).unwrap();
            let key = entry.0.clone();
            (entry.2.as_mut() as &mut dyn Any)
                .downcast_mut::<P::RenderInstance>()
                .unwrap()
                .update(ctx, data);
            key
        } else {
            let handle = self.inner.insert_with_key(|key| {
                (
                    Arc::new(key),
                    TypeId::of::<P::RenderInstance>(),
                    Box::new(P::RenderInstance::init(ctx, data)),
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
                self.last_frame_dropped.entry(*t_id).or_default().push(key);
            });
    }
}

/// Type erased [`RenderCommand`]
pub trait AnyRenderCommand: Any + RenderCommand + Send + Sync {}
impl<T: RenderCommand + Any + Send + Sync> AnyRenderCommand for T {}

impl RenderCommand for Vec<&dyn RenderCommand> {
    fn encode_compute_pass_command(&self, cpass: &mut wgpu::ComputePass) {
        for render_instance in self {
            render_instance.encode_compute_pass_command(cpass);
        }
    }
    fn encode_depth_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        for render_instance in self {
            render_instance.encode_depth_render_pass_command(rpass);
        }
    }
    fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        for render_instance in self {
            render_instance.encode_render_pass_command(rpass);
        }
    }
    fn debug(&self, ctx: &WgpuContext) {
        for render_instance in self {
            render_instance.debug(ctx);
        }
    }
}
