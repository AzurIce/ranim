pub mod fading;
pub mod transform;

use std::time;

use crate::{mobject::Mobject, pipeline::PipelineVertex, utils::rate_functions::smooth};

pub struct AnimationConfig {
    pub run_time: time::Duration,
    pub rate_func: Box<dyn Fn(f32) -> f32>,
    pub remove: bool,
}

impl Default for AnimationConfig {
    /// Default animation config
    /// - run_time: 1.0s
    /// - rate_func: linear
    /// - remove: false
    fn default() -> Self {
        Self {
            run_time: time::Duration::from_secs_f32(1.0),
            rate_func: Box::new(smooth),
            remove: false,
        }
    }
}

impl AnimationConfig {
    pub fn run_time(mut self, run_time: time::Duration) -> Self {
        self.run_time = run_time;
        self
    }
    
    pub fn rate_func(mut self, rate_func: Box<dyn Fn(f32) -> f32>) -> Self {
        self.rate_func = rate_func;
        self
    }

    pub fn remove(mut self) -> Self {
        self.remove = true;
        self
    }

    pub fn calc_frames(&self, fps: f32) -> usize {
        (self.run_time.as_secs_f32() * fps).ceil() as usize
    }
}

pub trait AnimationFunc<Vertex: PipelineVertex> {
    #[allow(unused)]
    fn pre_anim(&mut self, mobject: &mut Mobject<Vertex>) {}

    fn interpolate(&mut self, mobject: &mut Mobject<Vertex>, alpha: f32);

    #[allow(unused)]
    fn post_anim(&mut self, mobject: &mut Mobject<Vertex>) {}
}

/// A struct representing an animation
/// 
/// The creation of an animation takes the ownership of the mobject to be animated (which is called "the animated mobject"), and
/// during the animation, this mobject's properties will be modified, but keeps the same id.
/// 
/// An [`Animation`] doesn't plays itself, it just describe what an animation like.
/// To play an animation, should use [`crate::scene::Scene`]'s [`crate::scene::Scene::play`] method.
/// 
/// The scene will use [`Animation::func`]'s [`AnimationFunc::interpolate`] method to modify the mobject,
/// and then use [`crate::scene::Scene::add_mobject`] to update the mobject.
/// 
/// When the animation is done, the scene will return an [`Option<Mobject>`] according to
/// [`AnimationConfig::remove`].
/// If `remove` is `true`, the scene will remove the mobject from the scene and return `None`.
/// Otherwise, the scene will return the modified mobject and keep it in the scene.
pub struct Animation<Vertex: PipelineVertex> {
    /// The mobject to be animated, will take the ownership of it, and return by scene's [`crate::scene::Scene::play`] method
    pub mobject: Mobject<Vertex>,
    pub func: Box<dyn AnimationFunc<Vertex>>,
    pub config: AnimationConfig,
}

impl<Vertex: PipelineVertex> Animation<Vertex> {
    pub fn new(
        mobject: Mobject<Vertex>,
        func: impl AnimationFunc<Vertex> + 'static,
        config: AnimationConfig,
    ) -> Self {
        Self {
            mobject,
            func: Box::new(func),
            config,
        }
    }

    pub fn should_remove(&self) -> bool {
        self.config.remove
    }

    /// Modify the corresponding mobject according to the alpha value
    pub fn interpolate(&mut self, alpha: f32) {
        self.func.interpolate(&mut self.mobject, alpha);
    }
}

