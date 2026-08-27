//! Audio model: sources, the [`Sound`] composition leaf, and the static
//! [`AudioPlan`].
//!
//! Audio never enters the per-frame evaluation pipeline. A sound leaf composes
//! exactly like a visual animation on the declaration side (it is placed by
//! the same containers), but its content is resolved statically: [`seal`]
//! flattens every [`Sound`] leaf in the tree into an [`AudioPlan`] of absolute
//! master-clock spans, and the consumer (video writer, preview tooling) reads
//! that plan instead of pulling samples per frame.
//!
//! The leaf contract mirrors [`Eval`](crate::animation::eval::Eval): a sound's
//! content is a pure function of its own normalized progress, and all time
//! management (windows, rate functions, container remaps) lives on the cells.
//! What differs is the consumption shape: a mixer needs every sample exactly
//! once in order, so sources expose block reads
//! ([`SoundSource::read_span`]) instead of single-value pulls.
//!
//! [`seal`]: crate::RanimScene::seal

use std::{ops::Range, sync::Arc};

use crate::animation::AnimationInfo;

/// Master audio sample rate. All plans and mixed buffers live on this grid.
pub const AUDIO_SAMPLE_RATE: u32 = 48_000;

/// One interleaved stereo frame of `f32` samples.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct StereoFrame {
    /// Left channel sample.
    pub l: f32,
    /// Right channel sample.
    pub r: f32,
}

impl StereoFrame {
    /// Silence.
    pub const ZERO: Self = Self { l: 0.0, r: 0.0 };

    /// A frame with both channels set to `v`.
    pub fn splat(v: f32) -> Self {
        Self { l: v, r: v }
    }

    /// Scale both channels by `g`.
    #[must_use]
    pub fn scale(self, g: f32) -> Self {
        Self {
            l: self.l * g,
            r: self.r * g,
        }
    }

    /// Linear interpolation between two frames (`t` unclamped).
    #[must_use]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            l: self.l + (other.l - self.l) * t,
            r: self.r + (other.r - self.r) * t,
        }
    }
}

impl std::ops::Add for StereoFrame {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            l: self.l + rhs.l,
            r: self.r + rhs.r,
        }
    }
}

impl std::ops::AddAssign for StereoFrame {
    fn add_assign(&mut self, rhs: Self) {
        self.l += rhs.l;
        self.r += rhs.r;
    }
}

/// The content protocol of a sound leaf: normalized-progress block reads.
///
/// Content is a pure function of progress `alpha ∈ [0, 1]`, exactly like
/// [`Eval`](crate::animation::eval::Eval) leaves; the difference is delivery:
/// the mixer fills whole buffers, so a source must fill a slice covering a
/// normalized span in one call. Sources that only make sense as point queries
/// may rely on the [`SoundSource::sample_at`] default.
pub trait SoundSource: Send + Sync + 'static {
    /// Length of the content in seconds at unit playback rate.
    ///
    /// `None` means procedural content with no intrinsic length; the leaf then
    /// defaults to a one-second window, matching visual leaves.
    fn natural_secs(&self) -> Option<f64> {
        None
    }

    /// Fill `dst` with content covering the normalized span
    /// `[start_alpha, end_alpha)` linearly: sample `i` is content at
    /// `start_alpha + (end_alpha - start_alpha) * i / dst.len()`.
    ///
    /// Callers guarantee `0 <= start_alpha <= end_alpha <= 1`. The span is
    /// usually the whole leaf window, so this is where a source amortizes all
    /// per-call setup.
    fn read_span(&self, dst: &mut [StereoFrame], start_alpha: f64, end_alpha: f64);

    /// Content at a single normalized position.
    ///
    /// Default implementation forwards to a one-sample
    /// [`SoundSource::read_span`]; sources with cheap random access may
    /// override. This is the fallback path for non-linear time remaps.
    fn sample_at(&self, alpha: f64) -> StereoFrame {
        let mut buf = [StereoFrame::ZERO];
        let end = (alpha + 1e-9).min(1.0);
        self.read_span(&mut buf, alpha, end.max(alpha));
        buf[0]
    }
}

/// A procedural source: any `Fn(f64) -> StereoFrame` of normalized progress.
///
/// Progress is the only input; give the leaf a window with
/// `with_duration` and decide inside the closure what progress means.
pub struct Synth<F: Fn(f64) -> StereoFrame + Send + Sync + 'static>(F);

impl<F: Fn(f64) -> StereoFrame + Send + Sync + 'static> Synth<F> {
    /// Wrap a synthesis closure.
    pub fn new(f: F) -> Self {
        Self(f)
    }
}

impl<F: Fn(f64) -> StereoFrame + Send + Sync + 'static> SoundSource for Synth<F> {
    fn read_span(&self, dst: &mut [StereoFrame], start_alpha: f64, end_alpha: f64) {
        let n = dst.len().max(1) as f64;
        let span = end_alpha - start_alpha;
        for (i, out) in dst.iter_mut().enumerate() {
            *out = (self.0)(start_alpha + span * i as f64 / n);
        }
    }
}

/// A decoded PCM buffer (for example from a wav/mp3 file).
///
/// Stored at any sample rate: the normalized-progress contract makes playback
/// rate self-correcting (alpha maps to content position, not to samples), and
/// [`SoundSource::natural_secs`] keeps real-world duration.
pub struct PcmBuffer {
    frames: Arc<[StereoFrame]>,
    /// Duration of the buffer at unit playback rate.
    secs: f64,
}

impl PcmBuffer {
    /// Build from interleaved-ready frames and their real-world duration.
    pub fn new(frames: Arc<[StereoFrame]>, secs: f64) -> Self {
        Self { frames, secs }
    }

    /// Number of stored frames.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether the buffer holds no frames.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

impl SoundSource for PcmBuffer {
    fn natural_secs(&self) -> Option<f64> {
        (self.secs > 0.0).then_some(self.secs)
    }

    fn read_span(&self, dst: &mut [StereoFrame], start_alpha: f64, end_alpha: f64) {
        let len = self.frames.len();
        if len == 0 || dst.is_empty() {
            dst.fill(StereoFrame::ZERO);
            return;
        }
        let last = (len - 1) as f64;
        let n = dst.len().max(1) as f64;
        let span = end_alpha - start_alpha;
        for (i, out) in dst.iter_mut().enumerate() {
            let pos = (start_alpha + span * i as f64 / n).clamp(0.0, 1.0) * last;
            let idx = pos.floor() as usize;
            let frac = (pos - idx as f64) as f32;
            let a = self.frames[idx];
            let b = self.frames[(idx + 1).min(len - 1)];
            *out = a.lerp(b, frac);
        }
    }
}

/// Payload of a sound leaf, carried through the erased animation tree in
/// [`AnimationInfo`](crate::animation::AnimationInfo) for introspection and
/// consumed by the flattener.
#[derive(Clone)]
pub struct SoundInfo {
    /// The leaf's content source.
    pub source: Arc<dyn SoundSource>,
    /// Linear gain applied to this leaf's samples.
    pub gain: f32,
    /// The leaf window length this plan was built with (seconds).
    pub natural_secs: f64,
}

/// A sound leaf in the animation tree.
///
/// Constructed with [`sound`]; playback parameters (`with_duration`,
/// `with_rate_func`, `with_enabled`, `at`) come from the standard
/// [`AnimationExt`](crate::animation::AnimationExt)/[`Placeable`] machinery.
///
/// Semantics when composed:
/// - windows map to playback: `with_duration` stretches progress, i.e. it is
///   an explicit speed (and pitch) change — there is never an implicit
///   stretch;
/// - containers treat a sound like any leaf: stacks overlay (sum), sequences
///   concatenate, `lagged` staggers (window fills are silence);
/// - rate functions time-warp the content (tape-speed effect);
/// - `with_enabled(false)` mutes the whole subtree;
/// - a sound never extends the visual timeline: windows past
///   [`RanimScene::total_secs`](crate::RanimScene::seal) are truncated with a
///   warning at seal time.
pub struct Sound<S: SoundSource> {
    pub(crate) source: S,
    pub(crate) gain: f32,
}

impl<S: SoundSource> Sound<S> {
    /// Wrap a source as a sound leaf with unit gain.
    pub fn new(source: S) -> Self {
        Self { source, gain: 1.0 }
    }

    /// Set the linear gain of this leaf.
    pub fn with_gain(mut self, gain: f32) -> Self {
        assert!(
            gain.is_finite() && gain >= 0.0,
            "sound gain must be finite and non-negative"
        );
        self.gain = gain;
        self
    }
}

/// Wrap a source as a [`Sound`] leaf, ready for placement.
///
/// Returns the bare leaf so that source-level options (for example
/// [`Sound::with_gain`]) stay reachable; the timing builders
/// (`with_duration`, `with_rate_func`, `with_enabled`, `at`) come from the
/// standard [`AnimationExt`](crate::animation::AnimationExt)/[`Placeable`]
/// machinery.
pub fn sound<S: SoundSource + 'static>(source: S) -> Sound<S> {
    Sound::new(source)
}

/// One time-remap level on the path from the master clock to a leaf.
///
/// Maps an incoming-coordinate time `t` to the outgoing coordinate:
/// `t' = rate((t - start) / dur) * content_dur`, or nothing when `t` is
/// outside the level's active window.
#[derive(Clone, Copy, Debug)]
pub struct WarpLevel {
    /// Level window start in the incoming coordinate (seconds).
    pub start: f64,
    /// Level window duration in the incoming coordinate (seconds).
    pub dur: f64,
    /// Time-remap applied to normalized progress within the window.
    pub rate: fn(f64) -> f64,
    /// Multiplier from remapped progress to the outgoing coordinate.
    pub content_dur: f64,
}

impl WarpLevel {
    /// Whether this level's rate function behaves as the identity on
    /// `[0, 1]`, letting a whole chain fold into one affine map.
    ///
    /// Checked behaviorally (sampled identity) rather than by function
    /// address: the same function can live at different addresses across
    /// codegen units, while any user function that *is* the identity is
    /// eligible for the affine fast path regardless of its name.
    pub fn rate_is_identity(&self) -> bool {
        const SAMPLES: usize = 16;
        (0..=SAMPLES).all(|i| {
            let x = i as f64 / SAMPLES as f64;
            ((self.rate)(x) - x).abs() < 1e-9
        })
    }

    /// Apply this level to an incoming-coordinate time.
    ///
    /// Half-open window semantics (`start <= t < start + dur`), mirroring the
    /// runtime's `contains_sec` gate. Zero-duration levels never match.
    pub fn map(&self, t: f64) -> Option<f64> {
        if self.dur <= 0.0 || !(t >= self.start && t < self.start + self.dur) {
            return None;
        }
        Some((self.rate)((t - self.start) / self.dur) * self.content_dur)
    }
}

/// One flattened sound instance: a source occupying an absolute master-clock
/// window through a chain of time remaps.
#[derive(Clone, Debug)]
pub struct SpanEntry {
    /// Index into [`AudioPlan::sources`].
    pub source: u32,
    /// Linear gain applied to this entry.
    pub gain: f32,
    /// Absolute master-clock window (seconds). Exact when the warp chain is
    /// fully linear; the enclosing root window otherwise (the mixer gates
    /// per sample in that case).
    pub window: Range<f64>,
    /// Master-grid sample bounds (half-open), rounded from `window` and
    /// clipped to the scene length.
    pub start_sample: u64,
    /// Exclusive end sample bound (see [`SpanEntry::start_sample`]).
    pub end_sample: u64,
    /// Outer→inner remap chain; the final level yields the leaf's alpha.
    pub warp: Vec<WarpLevel>,
    /// Exact affine alpha map `alpha = a * t + b` when the whole chain is
    /// linear, enabling the block fast path in [`AudioPlan::mix`].
    pub affine: Option<(f64, f64)>,
}

impl SpanEntry {
    /// Map a master-clock time to this entry's leaf alpha, or `None` when
    /// `t` falls outside any active level. Mirrors the runtime evaluation
    /// (per-level window gates in the same coordinates).
    pub fn eval_alpha(&self, t: f64) -> Option<f64> {
        let mut cur = t;
        for level in &self.warp {
            cur = level.map(cur)?;
        }
        Some(cur.clamp(0.0, 1.0))
    }
}

/// The static audio layout of a sealed scene: every [`Sound`] leaf flattened
/// to absolute master-clock spans.
#[derive(Clone, Default)]
pub struct AudioPlan {
    /// Registry of sources referenced by [`SpanEntry::source`].
    pub sources: Vec<Arc<dyn SoundSource>>,
    /// Flattened sound instances.
    pub entries: Vec<SpanEntry>,
}

impl AudioPlan {
    /// Whether the scene declares no sound at all.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Render the plan into a master stereo buffer at
    /// [`AUDIO_SAMPLE_RATE`], summing overlapping entries in entry order.
    ///
    /// Deterministic: a pure function of the plan and `total_secs`, with a
    /// fixed summation order. Warns once if the mixed peak exceeds full
    /// scale (no automatic limiting).
    pub fn mix(&self, total_secs: f64) -> Vec<StereoFrame> {
        let sr = f64::from(AUDIO_SAMPLE_RATE);
        let master_len = (total_secs * sr).round().max(0.0) as usize;
        let mut out = vec![StereoFrame::ZERO; master_len];
        for entry in &self.entries {
            let s0 = (entry.start_sample as usize).min(master_len);
            let s1 = (entry.end_sample as usize).min(master_len);
            if s1 <= s0 {
                continue;
            }
            match entry.affine {
                Some((a, b)) if a.is_finite() && a != 0.0 => {
                    self.mix_linear(entry, s0, s1, a, b, &mut out);
                }
                _ => self.mix_scalar(entry, s0, s1, &mut out),
            }
        }
        let peak = out
            .iter()
            .fold(0.0f32, |m, f| m.max(f.l.abs()).max(f.r.abs()));
        if peak > 1.0 {
            tracing::warn!(
                "mixed audio peak {:.2} exceeds full scale; the output will clip",
                peak
            );
        }
        out
    }

    /// Fast path for fully linear chains: the alpha map is affine, so the
    /// whole window is one uniform-alpha block read.
    fn mix_linear(
        &self,
        entry: &SpanEntry,
        s0: usize,
        s1: usize,
        a: f64,
        b: f64,
        out: &mut [StereoFrame],
    ) {
        let sr = f64::from(AUDIO_SAMPLE_RATE);
        let t0 = s0 as f64 / sr;
        let t1 = s1 as f64 / sr;
        let (mut al, mut ar) = (a * t0 + b, a * t1 + b);
        if al.max(ar) <= 0.0 || al.min(ar) >= 1.0 {
            return;
        }
        let reversed = al > ar;
        if reversed {
            std::mem::swap(&mut al, &mut ar);
        }
        let lo = al.max(0.0);
        let hi = ar.min(1.0);
        if hi <= lo {
            return;
        }
        let mut scratch = vec![StereoFrame::ZERO; s1 - s0];
        self.sources[entry.source as usize].read_span(&mut scratch, lo, hi);
        if reversed {
            scratch.reverse();
        }
        for (i, frame) in scratch.into_iter().enumerate() {
            out[s0 + i] += frame.scale(entry.gain);
        }
    }

    /// General path: per-sample chain evaluation with per-level window gates
    /// (exactly the runtime's remap semantics), then a point query.
    fn mix_scalar(&self, entry: &SpanEntry, s0: usize, s1: usize, out: &mut [StereoFrame]) {
        let src = &self.sources[entry.source as usize];
        let inv_sr = 1.0 / f64::from(AUDIO_SAMPLE_RATE);
        for (i, slot) in out[s0..s1].iter_mut().enumerate() {
            let t = (s0 + i) as f64 * inv_sr;
            if let Some(alpha) = entry.eval_alpha(t) {
                *slot += src.sample_at(alpha).scale(entry.gain);
            }
        }
    }
}

/// The scene length is defined by its visual content: the largest end among
/// top-level subtrees that contain at least one non-sound leaf. Root-level
/// sound-only subtrees never extend the timeline; when a scene holds no
/// visuals at all, the largest sound end is used so audio-only scenes still
/// render.
pub(crate) fn scene_total_secs(infos: &[AnimationInfo]) -> f64 {
    let visual_end = infos
        .iter()
        .filter(|info| subtree_has_visual(info))
        .map(|info| info.range.end)
        .fold(0.0_f64, f64::max);
    if visual_end > 0.0 {
        return visual_end;
    }
    infos
        .iter()
        .map(|info| info.range.end)
        .fold(0.0_f64, f64::max)
}

fn subtree_has_visual(info: &AnimationInfo) -> bool {
    match (info.sound.is_some(), info.children.is_empty()) {
        // A pure sound leaf.
        (true, true) => false,
        // Any other leaf.
        (_, true) => true,
        // A container is visual-bearing when any child subtree is.
        (_, false) => info.children.iter().any(subtree_has_visual),
    }
}

/// Flatten every sound leaf of a sealed scene's built animation cells into an
/// [`AudioPlan`]. Pure: the same cells always produce the same plan.
pub fn flatten_plan(cells: &[crate::animation::AnimationCell], total_secs: f64) -> AudioPlan {
    let mut plan = AudioPlan::default();
    for cell in cells {
        walk_node(
            &cell.animation_info(),
            &mut Vec::new(),
            total_secs,
            &mut plan,
        );
    }
    plan
}

/// Depth-first walk mirroring the runtime's remap semantics. `chain` maps
/// absolute master-clock time into the coordinate system `info.range` is
/// expressed in (empty at the root).
fn walk_node(
    info: &AnimationInfo,
    chain: &mut Vec<WarpLevel>,
    total_secs: f64,
    plan: &mut AudioPlan,
) {
    if !info.enabled {
        return;
    }
    let level = WarpLevel {
        start: info.range.start,
        dur: info.range.end - info.range.start,
        rate: info.rate_func,
        content_dur: info.content_duration_secs,
    };
    if let Some(sound) = &info.sound {
        if level.dur <= 0.0 {
            tracing::debug!("sound leaf with non-positive window is silent");
            return;
        }
        // The leaf's eval receives progress directly (no content multiplier).
        chain.push(WarpLevel {
            content_dur: 1.0,
            ..level
        });
        add_entry(sound, chain, total_secs, plan);
        chain.pop();
        return;
    }
    if level.dur <= 0.0 {
        return;
    }
    chain.push(level);
    for child in &info.children {
        walk_node(child, chain, total_secs, plan);
    }
    chain.pop();
}

fn add_entry(sound: &SoundInfo, chain: &[WarpLevel], total_secs: f64, plan: &mut AudioPlan) {
    let source_id = plan.sources.len() as u32;
    plan.sources.push(Arc::clone(&sound.source));

    let all_linear = chain.iter().all(WarpLevel::rate_is_identity);
    // chain[0] lives in absolute master-clock coordinates: its window is a
    // conservative bound for non-linear chains.
    let root_window = chain
        .first()
        .map(|l| l.start..l.start + l.dur)
        .unwrap_or(0.0..total_secs);
    let (affine, window) = if all_linear {
        let affine = fold_affine(chain);
        // A collapsed map (zero content duration) stays on the scalar path;
        // its per-level gates reduce it to silence anyway.
        if affine.0.is_finite() && affine.0 > 1e-12 {
            (Some(affine), exact_window(affine, &root_window))
        } else {
            (None, root_window.clone())
        }
    } else {
        (None, root_window.clone())
    };

    if window.end > total_secs + 1e-9 || window.start < -1e-9 {
        tracing::warn!(
            "sound window {:?} extends past the visual timeline [0, {total_secs}]; truncating",
            window
        );
    }

    let sr = f64::from(AUDIO_SAMPLE_RATE);
    let master_len = (total_secs * sr).round().max(0.0);
    let start_sample = ((window.start * sr).round() as i64).clamp(0, master_len as i64) as u64;
    let end_sample =
        ((window.end * sr).round() as i64).clamp(start_sample as i64, master_len as i64) as u64;

    plan.entries.push(SpanEntry {
        source: source_id,
        gain: sound.gain,
        window,
        start_sample,
        end_sample,
        warp: chain.to_vec(),
        affine,
    });
}

/// Fold a fully linear chain into `alpha = a * t + b`.
fn fold_affine(chain: &[WarpLevel]) -> (f64, f64) {
    let (mut a, mut b) = (1.0, 0.0);
    for level in chain {
        let k = level.content_dur / level.dur;
        // t' = k * (t - start) applied to t = a_in * t + b_in.
        a *= k;
        b = (b - level.start) * k;
    }
    (a, b)
}

/// Invert the affine map at alpha 0 and 1; falls back to the root window when
/// degenerate (a == 0).
fn exact_window(affine: (f64, f64), root: &Range<f64>) -> Range<f64> {
    let (a, b) = affine;
    if !a.is_finite() || a.abs() < f64::EPSILON {
        return root.clone();
    }
    let (t0, t1) = ((-b / a), ((1.0 - b) / a));
    if a > 0.0 { t0..t1 } else { t1..t0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        animation::{AnimationExt, Placeable, eval::pure::Pure},
        core_item::vitem::VItem,
        utils::rate_functions::ease_in_quad,
        {RanimScene, lagged, seq, stack},
    };

    /// Constant source (0.25 both channels) with exact 1s natural length.
    struct Const(f64);
    impl SoundSource for Const {
        fn natural_secs(&self) -> Option<f64> {
            Some(self.0)
        }
        fn read_span(&self, dst: &mut [StereoFrame], start_alpha: f64, end_alpha: f64) {
            let n = dst.len().max(1) as f64;
            let span = end_alpha - start_alpha;
            for (i, out) in dst.iter_mut().enumerate() {
                let a = start_alpha + span * i as f64 / n;
                *out = StereoFrame::splat(0.25 + 0.25 * a as f32);
            }
        }
    }

    fn plan_of(scene: RanimScene) -> (AudioPlan, f64) {
        let sealed = scene.seal();
        let total = sealed.total_secs();
        (sealed.audio_plan().clone(), total)
    }

    #[test]
    fn synth_defaults_to_one_second_window() {
        let mut scene = RanimScene::new();
        scene.play(sound(Synth::new(|_| StereoFrame::splat(1.0))));
        let (plan, total) = plan_of(scene);
        assert_eq!(total, 1.0);
        assert_eq!(plan.entries.len(), 1);
        let e = &plan.entries[0];
        assert_eq!(e.window, 0.0..1.0);
        assert_eq!(e.start_sample, 0);
        assert_eq!(e.end_sample, u64::from(AUDIO_SAMPLE_RATE));
    }

    #[test]
    fn natural_secs_sets_the_window_and_total() {
        let mut scene = RanimScene::new();
        scene.play(sound(Const(2.5)));
        let (plan, total) = plan_of(scene);
        assert_eq!(total, 2.5);
        assert_eq!(plan.entries[0].window, 0.0..2.5);
    }

    #[test]
    fn at_offsets_the_absolute_window() {
        let mut scene = RanimScene::new();
        scene.play(sound(Const(1.0)).at(2.0));
        let (plan, _) = plan_of(scene);
        let e = &plan.entries[0];
        assert_eq!(e.window, 2.0..3.0);
        assert_eq!(e.start_sample, 2 * u64::from(AUDIO_SAMPLE_RATE));
    }

    #[test]
    fn window_past_total_secs_is_truncated() {
        let mut scene = RanimScene::new();
        scene.play(sound(Const(1.0)).at(0.5));
        // Shrink the visual timeline with a disabled short visual anim.
        scene.play(
            Pure::new(|_: f64| VItem::default())
                .with_duration(1.0)
                .with_enabled(false),
        );
        let (plan, total) = plan_of(scene);
        assert_eq!(total, 1.0);
        let e = &plan.entries[0];
        assert_eq!(e.window, 0.5..1.5);
        assert_eq!(e.end_sample, u64::from(AUDIO_SAMPLE_RATE));
        // Mixed output only reaches the master length.
        let mixed = plan.mix(total);
        assert_eq!(mixed.len(), AUDIO_SAMPLE_RATE as usize);
    }

    #[test]
    fn affine_path_eval_matches_manual_alpha() {
        let mut scene = RanimScene::new();
        scene.play(sound(Const(1.0)).at(2.0));
        let (plan, _) = plan_of(scene);
        let e = &plan.entries[0];
        assert!(e.affine.is_some());
        assert_eq!(e.eval_alpha(2.0), Some(0.0));
        assert_eq!(e.eval_alpha(2.5), Some(0.5));
        // Half-open window: the end instant is inactive.
        assert_eq!(e.eval_alpha(3.0), None);
        assert_eq!(e.eval_alpha(1.0), None);
        assert!(
            e.eval_alpha(2.9999999)
                .is_some_and(|a| (a - 1.0).abs() < 1e-6)
        );
    }

    #[test]
    fn nonlinear_rate_func_uses_scalar_path_and_maps_progress() {
        let mut scene = RanimScene::new();
        scene.play(sound(Const(1.0)).with_rate_func(ease_in_quad));
        let (plan, _) = plan_of(scene);
        let e = &plan.entries[0];
        assert!(e.affine.is_none());
        assert_eq!(e.eval_alpha(0.5), Some(ease_in_quad(0.5)));
        assert_eq!(e.eval_alpha(0.5), Some(0.25));
    }

    #[test]
    fn nested_container_chain_maps_like_the_runtime() {
        // seq of two 1s leaves, rate-warped by the seq, sound in the second.
        let mut scene = RanimScene::new();
        scene.play(
            seq![
                Pure::new(|_: f64| VItem::default()).with_duration(1.0),
                sound(Const(1.0)).with_duration(1.0),
            ]
            .with_duration(2.0),
        );
        let (plan, _) = plan_of(scene);
        assert_eq!(plan.entries.len(), 1);
        let e = &plan.entries[0];
        assert_eq!(e.window, 1.0..2.0);
        assert_eq!(e.eval_alpha(1.0), Some(0.0));
        assert_eq!(e.eval_alpha(1.5), Some(0.5));
        assert_eq!(e.eval_alpha(2.0), None);
    }

    #[test]
    fn mixing_sums_overlapping_entries_in_order() {
        let mut scene = RanimScene::new();
        scene.play(stack![
            sound(Const(1.0)),
            sound(Const(1.0)),
            sound(Const(1.0)).with_enabled(false),
        ]);
        let (plan, total) = plan_of(scene);
        let mixed = plan.mix(total);
        assert_eq!(mixed.len(), AUDIO_SAMPLE_RATE as usize);
        let frame = mixed[mixed.len() / 2];
        assert!((frame.l - 0.75).abs() < 1e-5); // two audible Const leaves sum
    }

    #[test]
    fn mix_fast_path_matches_scalar_path() {
        let mut scene = RanimScene::new();
        scene.play(stack![
            sound(Const(1.0)).with_duration(0.5).at(0.25),
            sound(Const(2.0)).with_gain(0.5).at(0.5),
        ]);
        let (mut plan, total) = plan_of(scene);
        let fast = plan.mix(total);
        for e in &mut plan.entries {
            e.affine = None; // force the scalar path
        }
        let scalar = plan.mix(total);
        assert_eq!(fast.len(), scalar.len());
        let max_diff = fast
            .iter()
            .zip(&scalar)
            .map(|(f, s)| (f.l - s.l).abs().max((f.r - s.r).abs()))
            .fold(0.0f32, f32::max);
        assert!(max_diff < 1e-4, "fast/scalar paths diverge by {max_diff}");
    }

    #[test]
    fn lagged_stagger_produces_offset_windows() {
        let mut scene = RanimScene::new();
        scene.play(lagged![
            0.5;
            sound(Const(1.0)).with_duration(1.0),
            sound(Const(1.0)).with_duration(1.0),
        ]);
        let (plan, _) = plan_of(scene);
        let mut windows: Vec<Range<f64>> = plan.entries.iter().map(|e| e.window.clone()).collect();
        windows.sort_by(|a, b| a.start.total_cmp(&b.start));
        assert_eq!(windows, vec![0.0..1.0, 0.5..1.5]);
    }

    #[test]
    fn disabled_scene_mutes_sounds() {
        let mut scene = RanimScene::new();
        scene.play(sound(Const(1.0)).with_enabled(false));
        let (plan, _) = plan_of(scene);
        assert!(plan.is_empty());
    }

    #[test]
    fn pcm_buffer_reads_are_alpha_normalized() {
        // 11 frames (values 0..=10) at "0.1s": position maps by alpha.
        let frames: Arc<[StereoFrame]> = (0..=10u32)
            .map(|i| StereoFrame::splat(i as f32))
            .collect::<Vec<_>>()
            .into();
        let src = PcmBuffer::new(frames, 0.1);
        let mut out = [StereoFrame::ZERO; 10];
        src.read_span(&mut out, 0.0, 1.0);
        // Alpha 0.5 lands on frame value 5 regardless of the frame rate.
        assert!((out[5].l - 5.0).abs() < 1e-4);
        assert!((src.sample_at(0.5).l - 5.0).abs() < 1e-4);
    }

    #[test]
    fn synth_read_span_spans_alpha_linearly() {
        let src = Synth::new(|a| StereoFrame::splat(a as f32));
        let mut out = [StereoFrame::ZERO; 5];
        src.read_span(&mut out, 0.5, 1.0);
        assert!((out[0].l - 0.5).abs() < 1e-5);
        assert!((out[4].l - 0.9).abs() < 1e-5);
    }
}
