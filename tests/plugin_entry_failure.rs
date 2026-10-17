//! Plugin constructor errors must be handled inside the library boundary.

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
    fn build(&self, _app: &mut ranim::App) {}
}

#[ranim::export_plugin]
fn create_plugin() -> FailingPlugin {
    std::panic::panic_any(Payload);
}

#[test]
fn native_export_drops_custom_panic_payload_before_returning_an_error() {
    unsafe extern "Rust" {
        #[link_name = "ranim_plugin"]
        fn exported_plugin() -> Result<Box<dyn ranim::Plugin>, String>;
    }

    let Err(error) = (unsafe { exported_plugin() }) else {
        panic!("plugin construction should fail");
    };
    assert!(error.contains("non-string payload"));
    assert!(PAYLOAD_DROPPED.load(Ordering::SeqCst));
}
