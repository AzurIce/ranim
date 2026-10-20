//! Reusable render configuration carried by a project.

use std::sync::Arc;

use bevy_app::{App, Plugin};

/// Deferred configuration for an independent render runtime.
///
/// Store this value in project resources and clone it into the plugin argument
/// of [`Renderer::new`](crate::Renderer::new). Creating or cloning a setup does
/// not initialize a GPU. Its callback runs for each renderer after shared GPU
/// resources and schedules have been installed.
#[derive(Clone)]
pub struct RenderSetup {
    configure: Arc<dyn Fn(&mut App) + Send + Sync>,
}

impl RenderSetup {
    /// Save repeatable configuration without running it.
    pub fn new(configure: impl Fn(&mut App) + Send + Sync + 'static) -> Self {
        Self {
            configure: Arc::new(configure),
        }
    }
}

impl Plugin for RenderSetup {
    fn build(&self, app: &mut App) {
        (self.configure)(app);
    }

    fn is_unique(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use bevy_app::SubApp;
    use bevy_ecs::prelude::Resource;

    use super::*;

    #[derive(Resource)]
    struct Configured(String);

    #[test]
    fn setup_is_deferred_and_reusable_across_render_apps() {
        let calls = Arc::new(AtomicUsize::new(0));
        let setup = RenderSetup::new({
            let calls = Arc::clone(&calls);
            let configuration = String::from("project configuration");
            move |app| {
                calls.fetch_add(1, Ordering::SeqCst);
                app.insert_resource(Configured(configuration.clone()));
            }
        });
        let cloned = setup.clone();
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        for setup in [setup, cloned] {
            let mut app = SubApp::new();
            app.add_plugins(setup);
            assert_eq!(
                app.world().resource::<Configured>().0,
                "project configuration"
            );
        }
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
