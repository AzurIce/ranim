//! Lightweight instrumentation and experimental upload strategies for the
//! `queue.write_buffer` paths, used to profile the per-frame full-buffer
//! upload cost. Enabled via `RANIM_PROFILE_UPLOAD`:
//!
//! - `count`        — record bytes/calls/CPU-time per buffer label (no behavior change)
//! - `skip-equal`   — additionally skip the upload when the payload is
//!                    byte-identical to the previous upload of the same buffer
//! - `dirty-ranges` — additionally upload only coalesced 256-byte-aligned dirty
//!                    ranges instead of the whole buffer
//!
//! All modes are no-ops (one relaxed `OnceLock` read per call) unless the env
//! var is set. This is profiling/experiment infrastructure, not a production
//! optimization: `skip-equal` intentionally skips GPU-written scratch buffers
//! (safe, they are fully rewritten by the compute pass) while buffers that
//! need CPU-side re-initialization each frame (`Merged ClipBoxes`) are
//! exempted from both strategies and always fully uploaded.

use std::{
    collections::BTreeMap,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU8, AtomicU64, Ordering},
    },
    time::Duration,
};

/// Upload instrumentation mode, seeded from `RANIM_PROFILE_UPLOAD` and
/// runtime-switchable via [`set_mode`] (e.g. from the preview profiler panel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UploadMode {
    #[default]
    Off,
    /// Record stats only.
    Count,
    /// Record stats and skip byte-identical whole-buffer uploads.
    SkipEqual,
    /// Record stats and upload only coalesced dirty 256-byte ranges.
    DirtyRanges,
}

impl UploadMode {
    fn from_env() -> Self {
        match std::env::var("RANIM_PROFILE_UPLOAD").as_deref() {
            Ok(v) if v == "1" || v == "count" || v == "true" => UploadMode::Count,
            Ok(v) if v == "skip-equal" || v == "skip_equal" => UploadMode::SkipEqual,
            Ok(v) if v == "dirty-ranges" || v == "dirty_ranges" => UploadMode::DirtyRanges,
            _ => UploadMode::Off,
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            UploadMode::Off => 0,
            UploadMode::Count => 1,
            UploadMode::SkipEqual => 2,
            UploadMode::DirtyRanges => 3,
        }
    }

    fn from_u8(v: u8) -> Self {
        match v {
            1 => UploadMode::Count,
            2 => UploadMode::SkipEqual,
            3 => UploadMode::DirtyRanges,
            _ => UploadMode::Off,
        }
    }

    pub fn enabled(self) -> bool {
        self != UploadMode::Off
    }
}

/// `u8::MAX` marks "not seeded from the environment yet".
static MODE: AtomicU8 = AtomicU8::new(u8::MAX);
/// Bumped on every [`set_mode`] so shadow copies can detect that they were
/// built under a different mode and must be re-established.
static MODE_GENERATION: AtomicU64 = AtomicU64::new(0);

/// The process-wide upload mode (seeded from `RANIM_PROFILE_UPLOAD` on first
/// use, overridable at runtime via [`set_mode`]).
pub fn mode() -> UploadMode {
    let raw = MODE.load(Ordering::Relaxed);
    if raw != u8::MAX {
        return UploadMode::from_u8(raw);
    }
    let seeded = UploadMode::from_env().as_u8();
    // On success compare_exchange yields the OLD value (the sentinel), so
    // the resolved mode is `seeded` when we won, or the concurrent writer's
    // value from the Err.
    let actual = match MODE.compare_exchange(u8::MAX, seeded, Ordering::Relaxed, Ordering::Relaxed)
    {
        Ok(_previous_sentinel) => seeded,
        Err(existing) => existing,
    };
    UploadMode::from_u8(actual)
}

/// Current mode generation; changes whenever [`set_mode`] is called.
pub(crate) fn mode_generation() -> u64 {
    MODE_GENERATION.load(Ordering::Relaxed)
}

/// Switch the process-wide upload mode at runtime (takes effect for uploads
/// after this call). Any shadow copies built under the previous mode are
/// invalidated and re-established with a full upload.
pub fn set_mode(new_mode: UploadMode) {
    // Resolve the unseeded sentinel first so the generation bump always
    // compares against a real mode.
    let _ = mode();
    MODE.store(new_mode.as_u8(), Ordering::Relaxed);
    MODE_GENERATION.fetch_add(1, Ordering::Relaxed);
}

/// Whether a mode needs the shadow copy maintained (`Count` does not).
pub(crate) fn needs_shadow(mode: UploadMode) -> bool {
    matches!(mode, UploadMode::SkipEqual | UploadMode::DirtyRanges)
}

#[derive(Default)]
struct Counters {
    /// Number of `set` calls (upload attempts), including skipped ones.
    calls: AtomicU64,
    /// Logical bytes the caller wanted uploaded (the full payload size).
    bytes: AtomicU64,
    /// Bytes actually handed to `queue.write_buffer` (0 when skipped).
    written_bytes: AtomicU64,
    /// CPU time spent inside the write calls (excluding diff time).
    cpu_ns: AtomicU64,
    /// CPU time spent diffing payloads against the shadow copy.
    diff_ns: AtomicU64,
}

fn registry() -> &'static Mutex<BTreeMap<&'static str, Counters>> {
    static REG: OnceLock<Mutex<BTreeMap<&'static str, Counters>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn label_or(label: Option<&'static str>) -> &'static str {
    label.unwrap_or("<unnamed>")
}

/// Record one upload (or skipped upload) for `label`.
pub(crate) fn record(
    label: Option<&'static str>,
    bytes: u64,
    written_bytes: u64,
    cpu_ns: u64,
    diff_ns: u64,
) {
    let mut reg = registry().lock().unwrap();
    let c = reg.entry(label_or(label)).or_default();
    c.calls.fetch_add(1, Ordering::Relaxed);
    c.bytes.fetch_add(bytes, Ordering::Relaxed);
    c.written_bytes.fetch_add(written_bytes, Ordering::Relaxed);
    c.cpu_ns.fetch_add(cpu_ns, Ordering::Relaxed);
    c.diff_ns.fetch_add(diff_ns, Ordering::Relaxed);
}

/// Snapshot (and reset) the per-label counters accumulated so far.
#[derive(Debug, Clone, Copy)]
pub struct UploadStats {
    pub calls: u64,
    pub bytes: u64,
    pub written_bytes: u64,
    pub cpu_time: Duration,
    pub diff_time: Duration,
}

/// Take and reset the accumulated stats, per buffer label.
pub fn take_stats() -> BTreeMap<&'static str, UploadStats> {
    let mut reg = registry().lock().unwrap();
    reg.iter_mut()
        .map(|(&label, c)| {
            let cpu_ns = c.cpu_ns.swap(0, Ordering::Relaxed);
            let diff_ns = c.diff_ns.swap(0, Ordering::Relaxed);
            (
                label,
                UploadStats {
                    calls: c.calls.swap(0, Ordering::Relaxed),
                    bytes: c.bytes.swap(0, Ordering::Relaxed),
                    written_bytes: c.written_bytes.swap(0, Ordering::Relaxed),
                    cpu_time: Duration::from_nanos(cpu_ns),
                    diff_time: Duration::from_nanos(diff_ns),
                },
            )
        })
        .collect()
}

/// Granularity of the dirty-range detection.
pub const RANGE_BLOCK: usize = 256;
/// Above this many coalesced ranges, a whole-buffer write wins on per-call overhead.
pub const MAX_RANGES: usize = 16;

/// Compare `new` against `old` in [`RANGE_BLOCK`]-byte blocks and return
/// coalesced dirty ranges `(offset, len)` covering every differing block.
/// Returns a single whole-buffer range when more than [`MAX_RANGES`] ranges
/// would be needed.
pub fn dirty_ranges(old: &[u8], new: &[u8]) -> Vec<(usize, usize)> {
    debug_assert_eq!(old.len(), new.len());
    let len = new.len();
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut block_start = 0;
    while block_start < len {
        let block_end = (block_start + RANGE_BLOCK).min(len);
        if new[block_start..block_end] != old[block_start..block_end] {
            match ranges.last_mut() {
                Some((_, end)) if *end == block_start => *end = block_end,
                _ => ranges.push((block_start, block_end)),
            }
            if ranges.len() > MAX_RANGES {
                return vec![(0, len)];
            }
        }
        block_start = block_end;
    }
    ranges
}

/// Buffers that must be fully re-uploaded every frame because the GPU
/// accumulates into them (atomic min/max) rather than overwriting.
fn is_gpu_accumulated(label: Option<&'static str>) -> bool {
    label == Some("Merged ClipBoxes")
}

/// Decide what to do for a payload under the current mode.
#[derive(Debug)]
pub(crate) enum UploadDecision {
    /// Skip the upload entirely (payload identical to the shadow).
    Skip,
    /// Write only these byte ranges.
    Ranges(Vec<(usize, usize)>),
    /// Write the whole payload.
    Full,
}

/// Diff `bytes` against `shadow` (the previously uploaded payload) according
/// to `mode`. Returns the decision plus the time spent diffing.
pub(crate) fn decide(
    mode: UploadMode,
    label: Option<&'static str>,
    shadow: Option<&[u8]>,
    bytes: &[u8],
) -> (UploadDecision, Duration) {
    if !mode.enabled() || mode == UploadMode::Count || is_gpu_accumulated(label) {
        return (UploadDecision::Full, Duration::ZERO);
    }
    let Some(prev) = shadow else {
        return (UploadDecision::Full, Duration::ZERO);
    };
    debug_assert_eq!(prev.len(), bytes.len());
    let start = std::time::Instant::now();
    let decision = match mode {
        UploadMode::SkipEqual => {
            if prev == bytes {
                UploadDecision::Skip
            } else {
                UploadDecision::Full
            }
        }
        UploadMode::DirtyRanges => {
            let ranges = dirty_ranges(prev, bytes);
            if ranges.len() == 1 && ranges[0].0 == 0 && ranges[0].1 == bytes.len() {
                UploadDecision::Full
            } else if ranges.is_empty() {
                UploadDecision::Skip
            } else {
                UploadDecision::Ranges(ranges)
            }
        }
        _ => UploadDecision::Full,
    };
    (decision, start.elapsed())
}
