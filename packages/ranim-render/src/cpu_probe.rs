//! Lightweight CPU timing spans, mirroring [`crate::upload_probe`] and the
//! GPU timer scopes: leaf-level named spans recorded into a thread-local
//! buffer, drained once per frame by the consumer (the preview profiler
//! panel).
//!
//! Spans are always recorded (a span costs two `Instant::now` reads, ~50ns)
//! and are **leaf-level only** — never nest them, so the frame total is the
//! sum of all spans without double counting. If nobody drains the buffer
//! (e.g. the CLI render worker thread), it self-clears at a span cap so
//! memory stays bounded.

use std::{cell::RefCell, time::Instant};

/// Upper bound on buffered spans; exceeding it clears the buffer (the
/// consumer is expected to drain every frame).
const MAX_SPANS: usize = 256;

thread_local! {
    static SPANS: RefCell<Vec<(&'static str, f64)>> = const { RefCell::new(Vec::new()) };
}

/// A scoped CPU timing span. Records `(label, ms)` into the thread-local
/// frame buffer when dropped.
#[must_use]
pub struct Span {
    label: &'static str,
    start: Instant,
}

impl Drop for Span {
    fn drop(&mut self) {
        let ms = self.start.elapsed().as_secs_f64() * 1e3;
        SPANS.with(|spans| {
            let mut spans = spans.borrow_mut();
            if spans.len() >= MAX_SPANS {
                spans.clear();
            }
            spans.push((self.label, ms));
        });
    }
}

/// Start a named leaf-level CPU span (records on scope exit):
///
/// ```ignore
/// let _span = cpu_probe::span("my_stage");
/// do_work();
/// ```
pub fn span(label: &'static str) -> Span {
    Span {
        label,
        start: Instant::now(),
    }
}

/// Drain the spans recorded on this thread since the last call.
pub fn take_frame() -> Vec<(&'static str, f64)> {
    SPANS.with(|spans| std::mem::take(&mut *spans.borrow_mut()))
}
