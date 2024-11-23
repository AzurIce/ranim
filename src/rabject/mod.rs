pub mod vmobject;

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{renderer::Renderer, utils::Id, RanimContext};

pub trait Rabject {
    type RenderResource;

    /// Used to initialize the render resource when the rabject is extracted
    fn init_render_resource(ctx: &mut RanimContext, rabject: &Self) -> Self::RenderResource;

    fn update_render_resource(
        ctx: &mut RanimContext,
        rabject: &Self,
        render_resource: &mut Self::RenderResource,
    );

    fn render(ctx: &mut RanimContext, render_resource: &Self::RenderResource);
}

pub struct RabjectWithId<T: Rabject> {
    id: Id,
    rabject: Arc<RwLock<T>>,
}

impl<T: Rabject> From<T> for RabjectWithId<T> {
    fn from(rabject: T) -> Self {
        Self {
            id: Id::new(),
            rabject: Arc::new(RwLock::new(rabject)),
        }
    }
}

impl<T: Rabject> RabjectWithId<T> {
    pub fn extract(&self, ctx: &mut RanimContext) -> ExtractedRabjectWithId<T> {
        ExtractedRabjectWithId {
            id: self.id,
            rabject: self.rabject.clone(),
            render_resource: T::init_render_resource(ctx, &self.rabject.read().unwrap()),
        }
    }

    pub fn get(&self) -> RwLockReadGuard<T> {
        self.rabject.read().unwrap()
    }

    pub fn get_mut(&self) -> RwLockWriteGuard<T> {
        self.rabject.write().unwrap()
    }
}

pub struct ExtractedRabjectWithId<T: Rabject> {
    id: Id,
    rabject: Arc<RwLock<T>>,
    render_resource: T::RenderResource,
}

impl<T: Rabject> ExtractedRabjectWithId<T> {
    /// The extracted rabject should be read-only
    pub fn get(&self) -> RwLockReadGuard<T> {
        self.rabject.read().unwrap()
    }
}
