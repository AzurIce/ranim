//! Plugin builds report owned errors to the host that installs them.

use std::sync::atomic::{AtomicBool, Ordering};

static PAYLOAD_DROPPED: AtomicBool = AtomicBool::new(false);

struct Payload;

impl Drop for Payload {
    fn drop(&mut self) {
        PAYLOAD_DROPPED.store(true, Ordering::SeqCst);
    }
}

struct FailingPlugin;

impl ranim::Plugin for FailingPlugin {
    fn build(&self, app: &mut ranim::App) {
        app.insert_resource(42_u32);
        std::panic::panic_any(Payload);
    }
}

#[ranim::export_plugin]
fn project_plugin() -> impl ranim::Plugin {
    FailingPlugin
}

#[test]
fn plugin_build_drops_custom_panic_payload_before_reporting_an_error() {
    unsafe extern "Rust" {
        #[link_name = "ranim_plugin"]
        fn exported_plugin() -> Result<Box<dyn ranim::Plugin>, String>;
    }

    let exported = unsafe { exported_plugin() }.unwrap();
    let mut host = ranim::App::new();
    let Err(error) = host.try_add_plugins(exported) else {
        panic!("plugin build should fail");
    };
    assert!(error.to_string().contains("non-string payload"));
    assert!(PAYLOAD_DROPPED.load(Ordering::SeqCst));
    assert_eq!(*host.resource::<u32>(), 42);
}
