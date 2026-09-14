//! Lightweight upload instrumentation for the `queue.write_buffer` paths,
//! recording bytes / calls / CPU time per buffer label. Enabled via
//! `RANIM_PROFILE_UPLOAD=1` (or `set_mode` at runtime, e.g. from the
//! preview profiler panel); a no-op (one atomic read per upload) unless
//! enabled.
//!
//! This is the stable counting core. Experimental upload strategies
//! (skip-identical / dirty-range uploads) live on a separate branch and
//! extend this module.

use std::{
    collections::BTreeMap,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU8, AtomicU64, Ordering},
    },
    time::Duration,
};

/// Upload instrumentation mode, seeded from `RANIM_PROFILE_UPLOAD` and
/// runtime-switchable via [`set_mode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UploadMode {
    #[default]
    Off,
    /// Record stats only.
    Count,
}

impl UploadMode {
    fn from_env() -> Self {
        match std::env::var("RANIM_PROFILE_UPLOAD").as_deref() {
            Ok(v) if v == "1" || v == "count" || v == "true" => UploadMode::Count,
            _ => UploadMode::Off,
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            UploadMode::Off => 0,
            UploadMode::Count => 1,
        }
    }

    fn from_u8(v: u8) -> Self {
        match v {
            1 => UploadMode::Count,
            _ => UploadMode::Off,
        }
    }

    pub fn enabled(self) -> bool {
        self != UploadMode::Off
    }
}

/// `u8::MAX` marks "not seeded from the environment yet".
static MODE: AtomicU8 = AtomicU8::new(u8::MAX);
static MODE_SEEDED: OnceLock<()> = OnceLock::new();

/// The process-wide upload mode (seeded from `RANIM_PROFILE_UPLOAD` on
/// first use, overridable at runtime via [`set_mode`]).
pub fn mode() -> UploadMode {
    if MODE_SEEDED.get().is_none() {
        let _ = MODE_SEEDED.set(());
        MODE.store(UploadMode::from_env().as_u8(), Ordering::Relaxed);
    }
    UploadMode::from_u8(MODE.load(Ordering::Relaxed))
}

/// Switch the process-wide upload mode at runtime (takes effect for
/// uploads after this call).
pub fn set_mode(new_mode: UploadMode) {
    mode(); // resolve the unseeded sentinel first
    MODE.store(new_mode.as_u8(), Ordering::Relaxed);
}

#[derive(Default)]
struct Counters {
    /// Number of `set` calls (upload attempts).
    calls: AtomicU64,
    /// Logical bytes the caller wanted uploaded (the full payload size).
    bytes: AtomicU64,
    /// Bytes actually handed to `queue.write_buffer`.
    written_bytes: AtomicU64,
    /// CPU time spent inside the write calls.
    cpu_ns: AtomicU64,
}

fn registry() -> &'static Mutex<BTreeMap<&'static str, Counters>> {
    static REG: OnceLock<Mutex<BTreeMap<&'static str, Counters>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn label_or(label: Option<&'static str>) -> &'static str {
    label.unwrap_or("<unnamed>")
}

/// Record one upload for `label`.
pub(crate) fn record(label: Option<&'static str>, bytes: u64, written_bytes: u64, cpu_ns: u64) {
    let mut reg = registry().lock().unwrap();
    let c = reg.entry(label_or(label)).or_default();
    c.calls.fetch_add(1, Ordering::Relaxed);
    c.bytes.fetch_add(bytes, Ordering::Relaxed);
    c.written_bytes.fetch_add(written_bytes, Ordering::Relaxed);
    c.cpu_ns.fetch_add(cpu_ns, Ordering::Relaxed);
}

/// Snapshot of the counters accumulated since the last [`take_stats`].
#[derive(Debug, Clone, Copy)]
pub struct UploadStats {
    pub calls: u64,
    pub bytes: u64,
    pub written_bytes: u64,
    pub cpu_time: Duration,
}

/// Take and reset the accumulated stats, per buffer label.
pub fn take_stats() -> BTreeMap<&'static str, UploadStats> {
    let mut reg = registry().lock().unwrap();
    reg.iter_mut()
        .map(|(&label, c)| {
            (
                label,
                UploadStats {
                    calls: c.calls.swap(0, Ordering::Relaxed),
                    bytes: c.bytes.swap(0, Ordering::Relaxed),
                    written_bytes: c.written_bytes.swap(0, Ordering::Relaxed),
                    cpu_time: Duration::from_nanos(c.cpu_ns.swap(0, Ordering::Relaxed)),
                },
            )
        })
        .collect()
}
