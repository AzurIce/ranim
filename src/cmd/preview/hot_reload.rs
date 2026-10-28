//! Bridge patch notifications to egui, including when playback is paused.

use std::sync::{
    Arc, Once,
    atomic::{AtomicBool, Ordering},
};

use eframe::egui;

struct Notification {
    pending: AtomicBool,
    ctx: egui::Context,
}

/// The registered handler only holds a weak reference: closing the preview
/// releases its egui context even though Subsecond retains handlers globally.
pub(super) struct HotReload(Arc<Notification>);

impl HotReload {
    pub(super) fn new(ctx: egui::Context) -> Self {
        let notification = Arc::new(Notification {
            pending: AtomicBool::new(false),
            ctx,
        });
        let weak = Arc::downgrade(&notification);
        dioxus_devtools::subsecond::register_handler(Arc::new(move || {
            if let Some(notification) = weak.upgrade() {
                notification.pending.store(true, Ordering::Release);
                notification.ctx.request_repaint();
            }
        }));

        // Register before connecting, so the first patch cannot be missed.
        static CONNECT: Once = Once::new();
        CONNECT.call_once(dioxus_devtools::connect_subsecond);
        Self(notification)
    }

    pub(super) fn take_pending(&self) -> bool {
        self.0.pending.swap(false, Ordering::AcqRel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notifications_coalesce_and_are_consumed_once() {
        // No dx connection is needed to test the UI-side mailbox.
        let reload = HotReload(Arc::new(Notification {
            pending: AtomicBool::new(false),
            ctx: egui::Context::default(),
        }));
        assert!(!reload.take_pending());
        reload.0.pending.store(true, Ordering::Release);
        reload.0.pending.store(true, Ordering::Release);
        assert!(reload.take_pending());
        assert!(!reload.take_pending());
    }
}
