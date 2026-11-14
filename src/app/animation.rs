//! Explicit plugin registration of animation metadata.

use bevy_app::{App, Plugin};
use ranim_core::animation::registry::{Animation, AnimationRegistry};

/// Register inspectable animation families in a project.
pub trait AnimationAppExt {
    /// Register a type family; this does not register a serialization codec.
    ///
    /// # Panics
    ///
    /// Panics on conflicting type identities or Rust paths.
    fn register_animation<T: Animation>(&mut self) -> &mut Self;
}

impl AnimationAppExt for App {
    fn register_animation<T: Animation>(&mut self) -> &mut Self {
        self.world_mut()
            .get_resource_or_init::<AnimationRegistry>()
            .register::<T>();
        self
    }
}

/// Register the built-in animation families enabled by this App's features.
///
/// Generic registrations describe a family, not a codec for every output type.
pub struct BuiltinAnimationsPlugin;

impl Plugin for BuiltinAnimationsPlugin {
    fn build(&self, app: &mut App) {
        use ranim_core::animation::{
            compose::{lagged::AnimLagged, sequence::AnimSequence, stack::AnimStack},
            eval::Static,
            sound::Sound,
        };
        app.register_animation::<AnimSequence>()
            .register_animation::<AnimStack>()
            .register_animation::<AnimLagged>()
            .register_animation::<Static<()>>()
            .register_animation::<Sound>();

        #[cfg(feature = "anims")]
        {
            use ranim_anims::{
                camera::Orbit,
                creation::{Create, UnCreate, Unwrite, Write},
                fading::{FadeIn, FadeOut},
                morph::Morph,
                rotating::Rotate,
            };
            use ranim_items::vitem::VItem;
            app.register_animation::<Orbit>()
                .register_animation::<Create>()
                .register_animation::<UnCreate>()
                .register_animation::<Write>()
                .register_animation::<Unwrite>()
                .register_animation::<FadeIn>()
                .register_animation::<FadeOut>()
                .register_animation::<Morph<VItem>>()
                .register_animation::<Rotate>();
        }
    }
}
