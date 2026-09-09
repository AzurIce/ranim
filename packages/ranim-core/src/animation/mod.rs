//! Pure animation evaluation and hierarchical type-erased composition.
//!
//! The runtime tree is a closed world of node content ([`NodeContent`]): the
//! time-combinator vocabulary (sequence, stack) plus the leaf forms.
//! Authoring is open above it — `AnimSequence`/`AnimStack`/`AnimLagged` are
//! build-time constructors (like [`CoreItem`](crate::core_item::CoreItem)'s
//! extraction boundary), user leaves are open through
//! [`Eval`], and combinators expressible as
//! placements (e.g. [`AnimLagged`]) desugar into the primitives in
//! [`Animation::build`].

use std::{any::type_name, ops::Range};

use crate::{
    audio::AudioTrack,
    core_item::{AnyExtractCoreItem, DynItem},
    utils::rate_functions::linear,
};

/// Evaluation protocols and author-facing adapters.
pub mod eval;

use eval::{Eval, EvalDyn};
use lagged::AnimLagged;
use sequence::AnimSequence;
use stack::AnimStack;

/// Audio leaf animation: a sound placed and composed like any animation.
pub mod sound;
pub use sound::Sound;

/// Dynamic lagged (staggered, end-filled) animation container.
pub mod lagged;
/// Dynamic sequential animation container.
pub mod sequence;
/// Dynamic overlay animation container.
pub mod stack;

/// Runtime animation content category used by preview tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationInfoKind {
    /// A typed evaluator without animation children.
    Eval,
    /// A sequential animation container.
    Sequence,
    /// An overlay animation container.
    Stack,
    /// A captured, type-erased static output batch.
    Static,
    /// An audio leaf: resolved into the audio plane at seal time.
    Audio,
}

/// Hierarchical runtime animation information used by preview tooling.
#[derive(Clone)]
pub struct AnimationInfo {
    /// Concrete evaluator or container type name.
    pub anim_name: String,
    /// Runtime content category.
    pub kind: AnimationInfoKind,
    /// Time range in the parent content's local coordinates.
    pub range: Range<f64>,
    /// Duration of this node's inner content before outer timing is applied.
    pub content_duration_secs: f64,
    /// Time remapping function applied by this node.
    pub rate_func: fn(f64) -> f64,
    /// Whether this node contributes values during evaluation.
    pub enabled: bool,
    /// The content step of iterative segments (`1/N` progress per integration
    /// step, `N` declared via `with_steps`); `None` for other nodes.
    pub sim_step: Option<f64>,
    /// Direct animation children in this node's content coordinates.
    pub children: Vec<AnimationInfo>,
}

/// What a runtime node IS — the closed vocabulary of the tree.
///
/// Each variant earns its seat by having non-desugarable evaluation
/// semantics (a combinator expressible as pure window placement belongs in
/// [`Animation::build`], not here):
///
/// - [`NodeContent::Sequence`] — exclusive selection: the LAST child containing
///   the content time evaluates (children placed with `.at()` may overlap);
/// - [`NodeContent::Stack`] — overlay: EVERY child containing the content time
///   evaluates and the outputs sum;
/// - [`NodeContent::Leaf`] — the open world: any user [`Eval`] implementation,
///   the only type-erased box in the tree;
/// - [`NodeContent::Static`] — a captured output batch replayed over a window;
/// - [`NodeContent::Audio`] — a sound leaf, consumed by the seal-time bake.
pub(in crate::animation) enum NodeContent {
    /// Exclusive succession: the LAST child containing the content time
    /// evaluates.
    Sequence(Vec<AnimNode>),
    /// Overlay: EVERY child containing the content time evaluates.
    Stack(Vec<AnimNode>),
    /// A typed evaluator (`Eval` implementation) — no children.
    Leaf(Box<dyn EvalDyn>),
    /// A captured, type-erased static output batch.
    Static(Vec<DynItem>),
    /// An audio leaf: its track resolves through the seal-time bake.
    ///
    /// Boxed: a track is the fattest payload in the tree, and sibling
    /// scanning (window checks over many nodes) is cache-bound — nodes stay
    /// slim, the track pays one indirection only when audible.
    Audio(Box<AudioTrack>),
}

impl NodeContent {
    fn info_kind(&self) -> AnimationInfoKind {
        match self {
            NodeContent::Sequence(_) => AnimationInfoKind::Sequence,
            NodeContent::Stack(_) => AnimationInfoKind::Stack,
            NodeContent::Leaf(_) => AnimationInfoKind::Eval,
            NodeContent::Static(_) => AnimationInfoKind::Static,
            NodeContent::Audio(_) => AnimationInfoKind::Audio,
        }
    }
}

/// A single runtime animation node: one closed [`NodeContent`] plus everything
/// every kind shares — the subtree, the content axis, and the timing shell
/// (window, rate, enable) in this node's parent's coordinates.
pub struct AnimNode {
    pub(in crate::animation) content: NodeContent,
    /// Length of this node's content time axis, in seconds: the sequence
    /// cursor or stack extent for containers, a sound's play window, `1.0`
    /// for bare visual leaves, `0.0` for statics.
    pub(in crate::animation) internal_time_secs: f64,
    pub(in crate::animation) rate_func: Option<fn(f64) -> f64>,
    pub(in crate::animation) time_range: Range<f64>,
    pub(in crate::animation) enabled: bool,
    pub(in crate::animation) anim_name: &'static str,
}

/// An affine map from global seconds to content seconds: `t = a + b·x`.
///
/// Composed by the seal-time audio bake ([`bake_audio`]) while every rate
/// function along the path is linear; a non-linear rate drops it.
#[derive(Clone, Copy)]
pub(crate) struct MixAff {
    a: f64,
    b: f64,
}

impl MixAff {
    pub(crate) const IDENTITY: Self = Self { a: 0.0, b: 1.0 };

    /// Compose `y = m_a + m_b·x` after `self`: `y = m_a + m_b·(a + b·x)`.
    fn then(self, m_a: f64, m_b: f64) -> Self {
        Self {
            a: m_a + m_b * self.a,
            b: m_b * self.b,
        }
    }

    /// The global time whose image is `y` (`b > 0` on every audio path).
    fn inv_y(&self, y: f64) -> f64 {
        (y - self.a) / self.b
    }
}

impl AnimNode {
    /// This node's children — only the container kinds have any; leaves
    /// read as the empty slice, so traversals stay uniform without storing
    /// an impossible empty `Vec` per leaf.
    pub(crate) fn children(&self) -> &[AnimNode] {
        match &self.content {
            NodeContent::Sequence(children) | NodeContent::Stack(children) => children,
            _ => &[],
        }
    }

    /// Global or parent-relative time range, depending on its containing plan.
    pub fn time_range(&self) -> Range<f64> {
        self.time_range.clone()
    }

    /// Duration in seconds.
    pub fn duration_secs(&self) -> f64 {
        self.time_range.end - self.time_range.start
    }

    pub(in crate::animation) fn shift_by(&mut self, offset_sec: f64) {
        self.time_range.start += offset_sec;
        self.time_range.end += offset_sec;
    }

    /// Whether this clip contributes a value.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Concrete evaluator type name captured before erasure.
    pub fn anim_name(&self) -> &str {
        self.anim_name
    }

    /// Whether the given scene time is inside this clip's inclusive range.
    pub fn active_at(&self, sec: f64) -> bool {
        sec >= self.time_range.start && sec <= self.time_range.end
    }

    /// Compute the rate-warped progress in this cell's local coordinates.
    ///
    /// The cell owns the time configuration (start, duration, rate function)
    /// and turns it into a reading; evaluators never see the configuration
    /// itself. A zero-duration cell reports `alpha == 1.0`.
    fn local_alpha(&self, sec: f64) -> f64 {
        let duration = self.duration_secs();
        let raw = if duration == 0.0 {
            1.0
        } else {
            (sec - self.time_range.start) / duration
        };
        self.rate_func.map_or(raw, |rate| rate(raw))
    }

    /// Evaluate this node at a time point — the ONLY time-management entry.
    ///
    /// Remaps the scene time to this node's local `alpha` (via its rate
    /// function), then evaluates the content at that progress (a pure query
    /// on `&self`). Direction management (forward vs backward reset+replay,
    /// how many `sim_step`s to integrate) is INTERNAL to each stateful node.
    /// Audio nodes evaluate to nothing here: their content resolves through
    /// the seal-time audio bake ([`bake_audio`]) instead of the frame
    /// pipeline.
    pub(crate) fn eval_at(&self, sec: f64, output: &mut Vec<DynItem>) {
        if !self.enabled || !self.active_at(sec) {
            return;
        }
        let alpha = self.local_alpha(sec);
        match &self.content {
            NodeContent::Sequence(children) => {
                let content = self.internal_time_secs * alpha;
                eval_sequence(children, content, self.internal_time_secs, output)
            }
            NodeContent::Stack(children) => {
                let content = self.internal_time_secs * alpha;
                eval_stack(children, content, self.internal_time_secs, output)
            }
            NodeContent::Leaf(eval) => eval.eval_into(alpha, output),
            NodeContent::Static(items) => output.extend(items.iter().cloned()),
            NodeContent::Audio(_) => {}
        }
    }

    /// Bake this subtree's audio into `pcm` (the whole timeline's interleaved
    /// stereo buffer, absolute frame indices), collecting what cannot be
    /// pre-mixed.
    ///
    /// The descent composes the affine global→content map (`aff`) while
    /// every rate along the path is linear; a linear sound leaf then knows
    /// its exact audible frame range and mixes it in one tight loop
    /// ([`AudioTrack::mix_span_into`]). A leaf behind any non-linear rate has
    /// no such map — its root-to-leaf path is pushed to `warp_paths` for the
    /// residual pass instead. The tree is never mutated.
    pub(crate) fn bake_into<'a>(
        &'a self,
        span: (f64, f64),
        aff: Option<MixAff>,
        path: &mut Vec<&'a Self>,
        warp_paths: &mut Vec<Vec<&'a Self>>,
        pcm: &mut [f32],
        sample_rate: f64,
    ) {
        if !self.enabled {
            return;
        }
        let lo = span.0.max(self.time_range.start);
        let hi = span.1.min(self.time_range.end);
        if lo >= hi {
            return;
        }
        let internal = self.internal_time_secs;
        let linear = self.rate_func.is_none();
        // This node's map in parent coordinates: t = m_a + m_b·x.
        let (m_a, m_b) = if linear {
            let m_b = internal / self.duration_secs();
            (-self.time_range.start * m_b, m_b)
        } else {
            (0.0, f64::NAN)
        };
        let new_span = if linear {
            (m_a + m_b * lo, m_a + m_b * hi)
        } else {
            (0.0, internal)
        };
        path.push(self);
        match &self.content {
            NodeContent::Audio(track) => {
                if let Some(aff) = aff.filter(|_| linear) {
                    // Audible where content time ∈ [0, play window): the
                    // affine inverts that range into global frames exactly.
                    // (ceil is the exclusive upper edge; the lower edge
                    // takes an epsilon against the seconds↔frames round
                    // trip — both conventions match `sample_at`'s half-open
                    // play window.)
                    let clo = new_span.0.max(0.0);
                    let chi = new_span.1.min(internal);
                    if clo < chi {
                        let total = aff.then(m_a, m_b);
                        let g_lo =
                            ((total.inv_y(clo) * sample_rate - 1e-9).ceil() as i64).max(0) as usize;
                        let g_hi = ((total.inv_y(chi) * sample_rate).ceil() as i64)
                            .min((pcm.len() / 2) as i64)
                            .max(0) as usize;
                        if g_lo < g_hi {
                            track.mix_span_into(total.a, total.b, g_lo, g_hi, sample_rate, pcm);
                        }
                    }
                } else {
                    // Warped path: keep it for the residual pass.
                    warp_paths.push(path.clone());
                }
            }
            // Containers recurse; leaves read as no children.
            _ => {
                let new_aff = if linear {
                    aff.map(|aff| aff.then(m_a, m_b))
                } else {
                    None
                };
                for child in self.children() {
                    child.bake_into(new_span, new_aff, path, warp_paths, pcm, sample_rate);
                }
            }
        }
        path.pop();
    }

    /// Whether any sound leaf lives in this subtree (enabled or not).
    pub(crate) fn has_audio(&self) -> bool {
        match &self.content {
            NodeContent::Audio(_) => true,
            _ => self.children().iter().any(AnimNode::has_audio),
        }
    }

    pub(in crate::animation) fn contains_sec(&self, sec: f64, parent_duration: f64) -> bool {
        self.time_range.contains(&sec) || (sec == parent_duration && sec == self.time_range.end)
    }

    pub(crate) fn animation_info(&self) -> AnimationInfo {
        AnimationInfo {
            anim_name: self.anim_name.to_string(),
            kind: self.content.info_kind(),
            range: self.time_range.clone(),
            content_duration_secs: self.internal_time_secs,
            rate_func: self.rate_func.unwrap_or(linear),
            enabled: self.enabled,
            sim_step: match &self.content {
                NodeContent::Leaf(eval) => eval.sim_step(),
                _ => None,
            },
            children: self
                .children()
                .iter()
                .map(AnimNode::animation_info)
                .collect(),
        }
    }
}

/// Sequence evaluation: the LAST child containing the content time runs
/// (children placed with `.at()` inside a sequence may overlap; the later
/// one wins — this exclusivity is what makes a sequence more than sugar for
/// a stack of placements).
fn eval_sequence(children: &[AnimNode], content_sec: f64, extent: f64, output: &mut Vec<DynItem>) {
    if let Some(child) = children
        .iter()
        .rev()
        .find(|child| child.contains_sec(content_sec, extent))
    {
        child.eval_at(content_sec, output);
    }
}

/// Stack evaluation: EVERY child containing the content time runs, outputs
/// sum.
fn eval_stack(children: &[AnimNode], content_sec: f64, extent: f64, output: &mut Vec<DynItem>) {
    for child in children {
        if child.contains_sec(content_sec, extent) {
            child.eval_at(content_sec, output);
        }
    }
}

/// The non-linear remainder of the audio plane, as a compact tree.
///
/// Built at bake time from the collected root-to-leaf paths of sound leaves
/// behind non-linear rates, sharing common prefixes so ancestors evaluate
/// once per sample. Every node carries only what the per-sample walk reads
/// — the timing shell and the track reference, ~40 bytes — because sibling
/// scanning is cache-bound: a warp container forces its children to be
/// window-checked per sample, and fat nodes would stream ~2× the bytes.
/// (Every residual node is enabled by construction — the bake skips
/// disabled subtrees.)
enum Residual<'a> {
    Leaf {
        window: Range<f64>,
        internal: f64,
        rate: Option<fn(f64) -> f64>,
        track: &'a AudioTrack,
    },
    Node {
        window: Range<f64>,
        internal: f64,
        rate: Option<fn(f64) -> f64>,
        children: Vec<Residual<'a>>,
    },
}

/// Construction-time residual: paths keyed by node identity, so shared
/// prefixes merge soundly; [`ResidualLink::compact`] then flattens it into
/// the walked form.
enum ResidualLink<'a> {
    Leaf(&'a AnimNode),
    Node {
        node: &'a AnimNode,
        children: Vec<ResidualLink<'a>>,
    },
}

impl<'a> ResidualLink<'a> {
    fn node(&self) -> &'a AnimNode {
        match self {
            ResidualLink::Leaf(node) | ResidualLink::Node { node, .. } => node,
        }
    }

    /// Insert a root-to-leaf `path` into the forest, reusing shared prefixes.
    fn insert(forest: &mut Vec<ResidualLink<'a>>, path: &[&'a AnimNode]) {
        let Some((&head, rest)) = path.split_first() else {
            return;
        };
        let slot = match forest
            .iter_mut()
            .find(|link| std::ptr::eq(link.node(), head))
        {
            Some(slot) => slot,
            None => {
                forest.push(ResidualLink::Node {
                    node: head,
                    children: Vec::new(),
                });
                forest.last_mut().expect("just pushed")
            }
        };
        match slot {
            ResidualLink::Leaf(_) => {}
            ResidualLink::Node { children, .. } => {
                if rest.is_empty() {
                    // The path's head is its only node: an audio leaf.
                    *slot = ResidualLink::Leaf(head);
                } else {
                    ResidualLink::insert(children, rest);
                }
            }
        }
    }

    /// Flatten into the compact walked form.
    fn compact(&self) -> Residual<'a> {
        let node = self.node();
        let window = node.time_range.clone();
        let internal = node.internal_time_secs;
        let rate = node.rate_func;
        match self {
            ResidualLink::Leaf(_) => match &node.content {
                NodeContent::Audio(track) => Residual::Leaf {
                    window,
                    internal,
                    rate,
                    track,
                },
                _ => unreachable!("warp paths only end at audio leaves"),
            },
            ResidualLink::Node { children, .. } => Residual::Node {
                window,
                internal,
                rate,
                children: children.iter().map(|c| c.compact()).collect(),
            },
        }
    }
}

impl Residual<'_> {
    /// This residual branch's stereo sample at scene time `x` — point
    /// semantics, identical to a full-tree walk restricted to warp paths.
    fn sample_at(&self, x: f64) -> [f32; 2] {
        let (window, internal, rate) = match self {
            Residual::Leaf {
                window,
                internal,
                rate,
                ..
            }
            | Residual::Node {
                window,
                internal,
                rate,
                ..
            } => (window, internal, rate),
        };
        if x < window.start || x >= window.end {
            return [0.0; 2];
        }
        let raw = (x - window.start) / (window.end - window.start);
        let own = internal * rate.map_or(raw, |rate| rate(raw));
        match self {
            Residual::Leaf { track, .. } => track.sample_at(own),
            Residual::Node { children, .. } => {
                let mut acc = [0.0; 2];
                for child in children {
                    let [l, r] = child.sample_at(own);
                    acc[0] += l;
                    acc[1] += r;
                }
                acc
            }
        }
    }
}

/// Bake the tree's whole audio plane over `[0, total_secs]` into one
/// interleaved stereo buffer at `sample_rate`.
///
/// Two passes, neither mutating the tree: (1) a single descent pre-mixes
/// every sound leaf whose path is entirely linear into `pcm`; (2) the
/// collected paths of everything behind non-linear rates are folded into a
/// [`Residual`] forest and walked once per output sample — shared ancestors
/// evaluate once, linear and silent branches are gone entirely.
pub(crate) fn bake_audio(animations: &[AnimNode], total_secs: f64, sample_rate: f64) -> Vec<f32> {
    if !animations.iter().any(AnimNode::has_audio) {
        return Vec::new();
    }
    let out_frames = (total_secs * sample_rate).ceil() as usize;
    let mut pcm = vec![0.0f32; out_frames * 2];

    let mut warp_paths: Vec<Vec<&AnimNode>> = Vec::new();
    let mut path = Vec::new();
    for cell in animations {
        cell.bake_into(
            (0.0, total_secs),
            Some(MixAff::IDENTITY),
            &mut path,
            &mut warp_paths,
            &mut pcm,
            sample_rate,
        );
    }

    let mut links: Vec<ResidualLink> = Vec::new();
    for cells in &warp_paths {
        ResidualLink::insert(&mut links, cells);
    }
    let forest: Vec<Residual> = links.iter().map(|l| l.compact()).collect();
    if !forest.is_empty() {
        for frame in 0..out_frames {
            let x = frame as f64 / sample_rate;
            let mut acc = [0.0f32; 2];
            for root in &forest {
                let [l, r] = root.sample_at(x);
                acc[0] += l;
                acc[1] += r;
            }
            pcm[frame * 2] += acc[0];
            pcm[frame * 2 + 1] += acc[1];
        }
    }
    pcm
}

/// A statically typed animation definition that can be lowered into a runtime animation.
pub trait Animation: Sized {
    /// Lower this definition into its local runtime representation.
    fn build(self) -> AnimNode;
}

/// Capability for animation definitions that have not been fixed in parent time coordinates.
///
/// This trait is used to constrain the anims to be inserted into [`AnimSequence`].
/// Only anims those are not placed can be inserted into it.
pub trait Placeable: Animation {
    /// Place this definition at an offset in its parent's local time coordinates.
    fn at(self, offset_sec: f64) -> At<Self> {
        At {
            inner: self,
            offset_sec,
        }
    }
}

/// Playback parameter builders for animations that have not been placed yet.
pub trait AnimationExt: Placeable {
    /// Change the animation's rate function.
    fn with_rate_func(self, rate_func: fn(f64) -> f64) -> Paramed<Self> {
        Paramed::new(self).with_rate_func(rate_func)
    }

    /// Change the animation's duration.
    fn with_duration(self, duration_secs: f64) -> Paramed<Self> {
        Paramed::new(self).with_duration(duration_secs)
    }

    /// Enable or disable this animation's output.
    fn with_enabled(self, enabled: bool) -> Paramed<Self> {
        Paramed::new(self).with_enabled(enabled)
    }
}

impl<A: Placeable> AnimationExt for A {}

impl<E> Placeable for E
where
    E: Eval + 'static,
    E::Output: AnyExtractCoreItem,
{
}
impl<E> Animation for E
where
    E: Eval + 'static,
    E::Output: AnyExtractCoreItem,
{
    fn build(self) -> AnimNode {
        AnimNode {
            content: NodeContent::Leaf(Box::new(self)),
            internal_time_secs: 1.0,
            anim_name: type_name::<E>(),
            rate_func: None,
            time_range: 0.0..1.0,
            enabled: true,
        }
    }
}

/// Playback parameters applied to an animation definition.
#[derive(Debug, Clone)]
pub(crate) struct AnimationParam {
    /// Time remapping function; `None` is the identity (linear) rate, kept
    /// structural so the mixing descent can compose affine maps.
    pub rate_func: Option<fn(f64) -> f64>,
    /// Optional duration override in seconds.
    pub duration_secs: Option<f64>,
    /// Whether this animation contributes a value.
    pub enabled: bool,
}

impl Default for AnimationParam {
    fn default() -> Self {
        Self {
            rate_func: None,
            duration_secs: None,
            enabled: true,
        }
    }
}

/// An animation definition with overridden playback parameters.
pub struct Paramed<A> {
    inner: A,
    param: AnimationParam,
}

impl<A> Paramed<A> {
    /// Wrap an animation without overriding its duration.
    pub(crate) fn new(inner: A) -> Self {
        Self {
            inner,
            param: AnimationParam::default(),
        }
    }

    /// Change the animation's rate function.
    pub fn with_rate_func(mut self, rate_func: fn(f64) -> f64) -> Self {
        self.param.rate_func = Some(rate_func);
        self
    }

    /// Change the animation's duration.
    pub fn with_duration(mut self, duration_secs: f64) -> Self {
        assert_valid_duration(duration_secs);
        self.param.duration_secs = Some(duration_secs);
        self
    }

    /// Enable or disable this animation's output.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.param.enabled = enabled;
        self
    }
}

impl<A: Placeable + 'static> Placeable for Paramed<A> {}
impl<A: Placeable + 'static> Animation for Paramed<A> {
    fn build(self) -> AnimNode {
        let mut cell = self.inner.build();
        if let Some(duration_secs) = self.param.duration_secs {
            cell.time_range = 0.0..duration_secs;
        }
        cell.rate_func = self.param.rate_func;
        cell.enabled = self.param.enabled;
        cell.anim_name = type_name::<A>();
        cell
    }
}

/// An animation fixed at an offset in its parent's time coordinates.
///
/// This is a terminal placement entry: it implements [`Animation`] but not
/// [`Placeable`], so playback parameters must be configured before calling
/// [`Placeable::at`].
pub struct At<A> {
    inner: A,
    offset_sec: f64,
}

impl<A: Animation> Animation for At<A> {
    fn build(self) -> AnimNode {
        let mut animation = self.inner.build();
        animation.shift_by(self.offset_sec);
        animation
    }
}

fn assert_valid_duration(duration_secs: f64) {
    assert!(
        duration_secs.is_finite() && duration_secs >= 0.0,
        "animation duration must be finite and non-negative"
    );
}

/// Build a static cell replaying already-sampled items over `time_range`.
pub(in crate::animation) fn static_cell(state: Vec<DynItem>, time_range: Range<f64>) -> AnimNode {
    AnimNode {
        content: NodeContent::Static(state),
        internal_time_secs: 0.0,
        rate_func: None,
        time_range,
        enabled: true,
        anim_name: "Static",
    }
}

/// Collect iterators of animations into containers.
pub trait AnimIterExt: Iterator + Sized {
    /// Collect the animations into an [`AnimStack`] (all at the same origin).
    fn into_stack(self) -> AnimStack
    where
        Self::Item: Animation + 'static,
    {
        self.collect()
    }

    /// Collect the animations into an [`AnimSequence`] (played in order).
    fn into_seq(self) -> AnimSequence
    where
        Self::Item: Placeable + 'static,
    {
        self.collect()
    }

    /// Collect the animations into an [`AnimLagged`] with the given stagger ratio.
    fn into_lagged(self, lag_ratio: f64) -> AnimLagged
    where
        Self::Item: Placeable + 'static,
    {
        let mut lagged = AnimLagged::new(lag_ratio);
        for animation in self {
            lagged.push(animation);
        }
        lagged
    }
}

impl<I: Iterator> AnimIterExt for I {}

/// Requirement for [`StaticAnim`].
pub trait StaticAnimRequirement: Clone + AnyExtractCoreItem {}

impl<T: Clone + AnyExtractCoreItem> StaticAnimRequirement for T {}

/// Convenience methods for zero-duration static animations.
pub trait StaticAnim: StaticAnimRequirement + Sized {
    /// Show this value.
    fn show(&self) -> Paramed<Static<Self>>;
    /// Hide this value.
    fn hide(&self) -> Paramed<Static<Self>>;
}

impl<T: StaticAnimRequirement + 'static> StaticAnim for T {
    fn show(&self) -> Paramed<Static<Self>> {
        Static(self.clone()).with_duration(0.0)
    }

    fn hide(&self) -> Paramed<Static<Self>> {
        Static(self.clone()).with_enabled(false).with_duration(0.0)
    }
}

/// A constant evaluator.
pub struct Static<T: Clone>(pub T);

impl<T: Clone> Eval for Static<T> {
    type Output = T;

    fn eval_alpha(&self, _alpha: f64) -> Self::Output {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{
        eval::{Eval, EvalExt, pure::Pure},
        lagged::LaggedFill,
        sequence::AnimSequence,
        stack::AnimStack,
    };
    use crate::{Extract, core_item::CoreItem, core_item::vitem::VItem, lagged, seq, stack};

    /// A stateless test double: a `VItem` shifted to a fixed x.
    struct ShiftX(f32);

    impl Eval for ShiftX {
        type Output = VItem;

        fn eval_alpha(&self, _alpha: f64) -> Self::Output {
            let mut item = VItem::default();
            item.points[0].x = self.0;
            item
        }
    }

    /// A stateless test double: x = offset + alpha.
    struct ShiftAlpha(f32);

    impl Eval for ShiftAlpha {
        type Output = VItem;

        fn eval_alpha(&self, alpha: f64) -> Self::Output {
            let mut item = VItem::default();
            item.points[0].x = self.0 + alpha as f32;
            item
        }
    }

    fn leaf(x: f32, duration: f64) -> impl Placeable {
        ShiftX(x).with_duration(duration)
    }

    fn progress_leaf(offset: f32) -> impl Placeable {
        ShiftAlpha(offset)
    }

    fn evaluated_xs(items: Vec<DynItem>) -> Vec<f32> {
        items
            .into_iter()
            .flat_map(|item| item.extract())
            .filter_map(|item| match item {
                CoreItem::VItem(item) => Some(item.points[0].x),
                CoreItem::CameraFrame(_) | CoreItem::MeshItem(_) => None,
            })
            .collect()
    }

    fn sampled_xs(animation: &AnimNode, sec: f64) -> Vec<f32> {
        let mut items = Vec::new();
        animation.eval_at(sec, &mut items);
        evaluated_xs(items)
    }

    #[test]
    fn at_offsets_the_built_animation() {
        let animation = leaf(1.0, 2.0).at(3.0).build();
        assert_eq!(animation.time_range(), 3.0..5.0);
    }

    #[test]
    fn eval_uses_linear_one_second_defaults() {
        let animation = Static(VItem::default()).build();
        assert_eq!(animation.time_range(), 0.0..1.0);
    }

    #[test]
    fn apply_alpha_to_writes_the_requested_progress_state() {
        let mut item = VItem::default();

        let animation = ShiftAlpha(0.0).apply_alpha_to(&mut item, 0.25);
        assert_eq!(item.points[0].x, 0.25);

        let animation = animation.apply_to(&mut item);
        assert_eq!(item.points[0].x, 1.0);
        assert_eq!(animation.eval_alpha(0.5).points[0].x, 0.5);
    }

    #[test]
    fn pure_wraps_a_closure_into_eval() {
        let mut item = VItem::default();
        Pure::new(|alpha: f64| {
            let mut item = VItem::default();
            item.points[0].x = alpha as f32;
            item
        })
        .apply_alpha_to(&mut item, 0.5);
        assert_eq!(item.points[0].x, 0.5);
    }

    #[test]
    fn parametrized_sequence_remaps_the_group_timeline() {
        use crate::utils::rate_functions::ease_in_quad;

        let animation = seq![progress_leaf(0.0), progress_leaf(10.0)]
            .with_duration(4.0)
            .with_rate_func(ease_in_quad);
        let animation = animation.build();

        assert_eq!(animation.time_range(), 0.0..4.0);
        assert_eq!(sampled_xs(&animation, 2.0), vec![0.5]);
        let info = animation.animation_info();
        assert_eq!(info.range, 0.0..4.0);
        assert_eq!(info.content_duration_secs, 2.0);
        assert_eq!(info.children.len(), 2);
        assert_eq!(info.children[0].range, 0.0..1.0);
        assert_eq!(info.children[1].range, 1.0..2.0);
    }

    #[test]
    fn seq_uses_child_durations() {
        let sequence = seq![leaf(1.0, 2.0), leaf(2.0, 3.0)];
        assert_eq!(sequence.built_animations()[0].time_range(), 0.0..2.0);
        assert_eq!(sequence.built_animations()[1].time_range(), 2.0..5.0);
        assert_eq!(sequence.at(5.0).build().time_range(), 5.0..10.0);
    }

    #[test]
    fn stack_accepts_plain_and_positioned_children() {
        let animation = stack![leaf(1.0, 2.0), leaf(2.0, 3.0).at(1.0)];
        assert_eq!(animation.duration_secs(), 4.0);
        assert_eq!(animation.built_animations()[0].time_range(), 0.0..2.0);
        assert_eq!(animation.built_animations()[1].time_range(), 1.0..4.0);
        assert_eq!(animation.at(10.0).build().time_range(), 10.0..14.0);
    }

    #[test]
    fn composition_macros_build_dynamic_containers_without_an_arity_limit() {
        let empty_sequence: AnimSequence = seq![];
        let empty_stack: AnimStack = stack![];
        assert_eq!(empty_sequence.duration_secs(), 0.0);
        assert_eq!(empty_stack.duration_secs(), 0.0);

        let sequence: AnimSequence = seq![
            leaf(1.0, 1.0),
            leaf(2.0, 1.0),
            leaf(3.0, 1.0),
            leaf(4.0, 1.0),
            leaf(5.0, 1.0),
            leaf(6.0, 1.0),
            leaf(7.0, 1.0),
            leaf(8.0, 1.0),
            leaf(9.0, 1.0),
        ];
        assert_eq!(sequence.duration_secs(), 9.0);
        assert_eq!(sequence.built_animations().len(), 9);

        let stack: AnimStack = stack![
            leaf(1.0, 1.0),
            leaf(2.0, 2.0),
            leaf(3.0, 3.0),
            leaf(4.0, 4.0),
            leaf(5.0, 5.0),
            leaf(6.0, 6.0),
            leaf(7.0, 7.0),
            leaf(8.0, 8.0),
            leaf(9.0, 9.0),
        ];
        assert_eq!(stack.duration_secs(), 9.0);
        assert_eq!(stack.built_animations().len(), 9);
    }

    #[test]
    fn sequence_can_be_repositioned_after_erasure() {
        let mut sequence = AnimSequence::new();
        sequence
            .push(leaf(1.0, 2.0))
            .forward(1.0)
            .push(leaf(2.0, 1.0));
        let animation = sequence.at(10.0).build();
        let info = animation.animation_info();
        assert_eq!(info.range, 10.0..14.0);
        assert_eq!(info.children[0].range, 0.0..2.0);
        assert_eq!(info.children[1].range, 3.0..4.0);
    }

    #[test]
    fn hold_samples_only_animations_active_before_the_cursor() {
        let mut sequence = AnimSequence::new();
        sequence
            .push(stack![leaf(1.0, 1.0), leaf(2.0, 2.0)])
            .hold(1.0);

        assert_eq!(sequence.built_animations().len(), 2);
        assert_eq!(sampled_xs(&sequence.built_animations()[1], 2.5), vec![2.0]);
    }

    #[test]
    fn repeated_hold_creates_adjacent_static_animations() {
        let mut sequence = AnimSequence::new();
        sequence.push(leaf(3.0, 1.0)).hold(1.0).hold(2.0);

        assert_eq!(sequence.cursor_sec(), 4.0);
        assert_eq!(sequence.built_animations().len(), 3);
        assert_eq!(sequence.built_animations()[1].time_range(), 1.0..2.0);
        assert_eq!(sequence.built_animations()[2].time_range(), 2.0..4.0);
        assert_eq!(sampled_xs(&sequence.built_animations()[2], 3.5), vec![3.0]);
    }

    #[test]
    fn repeated_hold_replays_dyn_items_without_nesting_the_output_batch() {
        let mut sequence = AnimSequence::new();
        sequence
            .push(stack![leaf(1.0, 1.0), leaf(2.0, 1.0)])
            .hold(1.0)
            .hold(1.0);

        let mut first_hold = Vec::new();
        sequence.built_animations()[1].eval_at(1.5, &mut first_hold);
        let mut second_hold = Vec::new();
        sequence.built_animations()[2].eval_at(2.5, &mut second_hold);

        assert_eq!(first_hold.len(), 2);
        assert_eq!(second_hold.len(), 2);
        assert_eq!(evaluated_xs(second_hold), vec![1.0, 2.0]);
    }

    #[test]
    fn forward_does_not_hold_the_previous_state() {
        let mut sequence = AnimSequence::new();
        sequence.push(leaf(4.0, 1.0)).forward(1.0).hold(1.0);

        assert_eq!(sequence.cursor_sec(), 3.0);
        assert_eq!(sequence.built_animations().len(), 1);
    }

    #[test]
    fn hold_uses_the_sequences_final_evaluation() {
        let mut shown = VItem::default();
        shown.points[0].x = 5.0;

        let mut hidden = AnimSequence::new();
        hidden.push(leaf(1.0, 1.0)).push(shown.hide()).hold(1.0);
        assert_eq!(hidden.built_animations().len(), 2);
        let mut hidden_items = Vec::new();
        hidden.built_animations()[1].eval_at(1.0, &mut hidden_items);
        assert!(hidden_items.is_empty());

        let mut restored = AnimSequence::new();
        restored.push(leaf(1.0, 1.0)).push(shown.show()).hold(1.0);
        assert_eq!(sampled_xs(&restored.built_animations()[2], 1.5), vec![5.0]);
    }

    #[test]
    fn nested_sequences_keep_their_own_final_evaluation() {
        let mut shown = VItem::default();
        shown.points[0].x = 7.0;

        let inner = seq![leaf(1.0, 1.0), shown.show()];
        let mut outer = AnimSequence::new();
        outer.push(inner).hold(1.0);

        assert_eq!(outer.built_animations().len(), 2);
        assert_eq!(sampled_xs(&outer.built_animations()[1], 1.5), vec![7.0]);

        let hidden_inner = seq![leaf(1.0, 1.0), shown.hide()];
        let mut hidden_outer = AnimSequence::new();
        hidden_outer.push(hidden_inner).hold(1.0);
        assert_eq!(hidden_outer.built_animations().len(), 1);
    }

    #[test]
    fn dynamic_stack_keeps_children_at_the_same_origin() {
        let mut stack = AnimStack::new();
        stack.push(leaf(1.0, 1.0)).push(leaf(2.0, 3.0));

        assert_eq!(stack.duration_secs(), 3.0);
        assert_eq!(stack.built_animations()[0].time_range(), 0.0..1.0);
        assert_eq!(stack.built_animations()[1].time_range(), 0.0..3.0);
    }

    #[test]
    fn sequence_extend_appends_direct_children_and_preserves_local_gaps() {
        let mut source = AnimSequence::new();
        source
            .push(leaf(2.0, 1.0))
            .forward(2.0)
            .push(leaf(3.0, 1.0));

        let mut sequence = AnimSequence::new();
        sequence.push(leaf(1.0, 2.0)).extend(source);

        assert_eq!(sequence.cursor_sec(), 6.0);
        assert_eq!(sequence.built_animations().len(), 3);
        assert_eq!(sequence.built_animations()[0].time_range(), 0.0..2.0);
        assert_eq!(sequence.built_animations()[1].time_range(), 2.0..3.0);
        assert_eq!(sequence.built_animations()[2].time_range(), 5.0..6.0);
    }

    #[test]
    fn stack_extend_appends_direct_children() {
        let source = stack![leaf(2.0, 1.0), leaf(3.0, 2.0).at(1.0)];
        let mut stack = stack![leaf(1.0, 4.0)];
        stack.extend(source);

        assert_eq!(stack.duration_secs(), 4.0);
        assert_eq!(stack.built_animations().len(), 3);
        assert_eq!(stack.built_animations()[1].time_range(), 0.0..1.0);
        assert_eq!(stack.built_animations()[2].time_range(), 1.0..3.0);
    }

    #[test]
    fn lagged_staggers_children_by_ratio_of_previous_durations() {
        let lagged = lagged![0.5; leaf(1.0, 1.0), leaf(2.0, 2.0), leaf(3.0, 1.0)];
        assert_eq!(lagged.built_animations()[0].time_range(), 0.0..1.0);
        assert_eq!(lagged.built_animations()[1].time_range(), 0.5..2.5);
        assert_eq!(lagged.built_animations()[2].time_range(), 1.5..2.5);
        assert_eq!(lagged.duration_secs(), 2.5);

        // ratio 1.0 is a sequence, ratio 0.0 is a stack.
        let as_sequence = lagged![1.0; leaf(1.0, 1.0), leaf(2.0, 1.0)];
        assert_eq!(as_sequence.built_animations()[1].time_range(), 1.0..2.0);
        let as_stack = lagged![0.0; leaf(1.0, 1.0), leaf(2.0, 2.0)];
        assert_eq!(as_stack.built_animations()[1].time_range(), 0.0..2.0);
        assert_eq!(as_stack.duration_secs(), 2.0);
    }

    #[test]
    fn lagged_fills_window_edges_with_static_cells() {
        let lagged = lagged![
            0.5;
            progress_leaf(0.0).with_duration(1.0),
            progress_leaf(10.0).with_duration(1.0),
        ];
        let animation = lagged.build();
        assert_eq!(animation.time_range(), 0.0..1.5);

        // Before the second child's start: it shows its initial state (Hold leading).
        assert_eq!(sampled_xs(&animation, 0.25), vec![0.25, 10.0]);
        // After the first child's end: it holds its final state (Hold trailing).
        assert_eq!(sampled_xs(&animation, 1.0), vec![1.0, 10.5]);
        // At the container's end: both hold their final states.
        assert_eq!(sampled_xs(&animation, 1.5), vec![1.0, 11.0]);
    }

    #[test]
    fn lagged_with_empty_leading_renders_nothing_before_start() {
        let animation = lagged![
            0.5;
            progress_leaf(0.0).with_duration(1.0),
            progress_leaf(10.0).with_duration(1.0),
        ]
        .with_leading(LaggedFill::Empty)
        .build();

        // Before the second child's start: only the first renders.
        assert_eq!(sampled_xs(&animation, 0.25), vec![0.25]);
        // Trailing still holds by default.
        assert_eq!(sampled_xs(&animation, 1.0), vec![1.0, 10.5]);
    }

    #[test]
    fn lagged_fill_structure_is_materialized_as_per_item_tracks() {
        let animation = lagged![
            0.5;
            progress_leaf(0.0).with_duration(1.0),
            progress_leaf(10.0).with_duration(1.0),
        ]
        .build();
        let info = animation.animation_info();
        // Each item becomes a sequence track spanning the whole extent:
        // [leading fill][anim][trailing fill] (empty fills skipped).
        assert_eq!(info.children.len(), 2);

        let first = &info.children[0];
        assert_eq!(first.kind, AnimationInfoKind::Sequence);
        assert_eq!(first.range, 0.0..1.5);
        assert_eq!(first.children.len(), 2);
        assert_eq!(first.children[0].kind, AnimationInfoKind::Eval);
        assert_eq!(first.children[0].range, 0.0..1.0);
        assert_eq!(first.children[1].kind, AnimationInfoKind::Static);
        assert_eq!(first.children[1].range, 1.0..1.5);

        let second = &info.children[1];
        assert_eq!(second.kind, AnimationInfoKind::Sequence);
        assert_eq!(second.range, 0.0..1.5);
        assert_eq!(second.children.len(), 2);
        assert_eq!(second.children[0].kind, AnimationInfoKind::Static);
        assert_eq!(second.children[0].range, 0.0..0.5);
        assert_eq!(second.children[1].kind, AnimationInfoKind::Eval);
        assert_eq!(second.children[1].range, 0.5..1.5);
    }

    #[test]
    fn lagged_child_ending_with_hide_stays_hidden_after_its_window() {
        let mut shown = VItem::default();
        shown.points[0].x = 2.0;

        let animation = lagged![0.5; seq![leaf(2.0, 1.0), shown.hide()]].build();
        assert_eq!(animation.time_range(), 0.0..1.0);

        assert_eq!(sampled_xs(&animation, 0.5), vec![2.0]);
        // The seq ends with a hide cell: after the window the item stays hidden.
        let mut items = Vec::new();
        animation.eval_at(1.0, &mut items);
        assert!(items.is_empty());
    }

    #[test]
    fn animations_collect_into_containers() {
        let stack: AnimStack = vec![leaf(1.0, 1.0), leaf(2.0, 2.0)].into_iter().collect();
        assert_eq!(stack.duration_secs(), 2.0);
        assert_eq!(stack.built_animations()[1].time_range(), 0.0..2.0);

        let sequence: AnimSequence = vec![leaf(1.0, 1.0), leaf(2.0, 2.0)].into_iter().collect();
        assert_eq!(sequence.duration_secs(), 3.0);
        assert_eq!(sequence.built_animations()[1].time_range(), 1.0..3.0);

        let lagged = vec![leaf(1.0, 1.0), leaf(2.0, 1.0)]
            .into_iter()
            .into_lagged(0.5);
        assert_eq!(lagged.built_animations()[1].time_range(), 0.5..1.5);
        assert_eq!(lagged.duration_secs(), 1.5);

        let stack = vec![leaf(1.0, 2.0)].into_iter().into_stack();
        assert_eq!(stack.duration_secs(), 2.0);
        let sequence = vec![leaf(1.0, 1.0), leaf(2.0, 1.0)].into_iter().into_seq();
        assert_eq!(sequence.duration_secs(), 2.0);
    }

    // MARK: Sound leaves in the tree

    use crate::RanimScene;
    use crate::audio::{AudioClip, MASTER_SAMPLE_RATE};
    use crate::utils::rate_functions::ease_in_quad;
    use sound::Sound;

    fn tone(secs: f64) -> AudioClip {
        // A constant-amplitude clip: probes never land on a zero crossing.
        let pcm = vec![0.5f32; (secs * 48_000.0) as usize];
        AudioClip::from_pcm(pcm, 48_000, 1)
    }

    /// Mix the scene's audio and report (total, first, last) in seconds —
    /// total scene length and the first/last sample-seconds with audible
    /// energy.
    fn audible_region(scene: RanimScene) -> (f64, f64, f64) {
        let sealed = scene.seal();
        let total = sealed.total_secs();
        let evaluator = sealed.into_evaluator(120.0);
        let buf = evaluator.mix_audio(total, MASTER_SAMPLE_RATE);
        let first = buf
            .iter()
            .position(|s| s.abs() > 1e-4)
            .expect("expected audible samples");
        let last = buf.len()
            - 1
            - buf
                .iter()
                .rev()
                .position(|s| s.abs() > 1e-4)
                .expect("non-empty");
        (
            total,
            first as f64 / 2.0 / MASTER_SAMPLE_RATE as f64,
            last as f64 / 2.0 / MASTER_SAMPLE_RATE as f64,
        )
    }

    #[test]
    fn sound_in_sequence_mixes_to_its_window() {
        let mut scene = RanimScene::new();
        scene.play(seq![leaf(1.0, 1.0), Sound::new(tone(2.0))]);

        let (total, start, end) = audible_region(scene);
        assert!((total - 3.0).abs() < 1e-9);
        assert!((start - 1.0).abs() < 0.01);
        assert!((end - 3.0).abs() < 0.01);
    }

    #[test]
    fn sound_frames_push_no_items() {
        let mut scene = RanimScene::new();
        scene.play(Sound::new(tone(2.0)));
        let sealed = scene.seal();

        assert_eq!(sealed.eval_at_sec(1.0).count(), 0);
    }

    #[test]
    fn at_placed_sound_shifts_the_window() {
        let mut scene = RanimScene::new();
        scene.play(stack![Sound::new(tone(1.0)).at(2.0)]);
        let (_, start, end) = audible_region(scene);
        assert!((start - 2.0).abs() < 0.01);
        assert!((end - 3.0).abs() < 0.01);
    }

    #[test]
    fn disabled_sound_is_excluded() {
        let mut scene = RanimScene::new();
        scene.play(stack![Sound::new(tone(1.0)).with_enabled(false)]);
        let evaluator = scene.seal().into_evaluator(120.0);
        assert!(evaluator.has_audio());
        assert!(
            evaluator
                .mix_audio(1.0, MASTER_SAMPLE_RATE)
                .iter()
                .all(|s| s.abs() < 1e-4)
        );
    }

    #[test]
    fn scene_without_sound_has_no_audio() {
        let mut scene = RanimScene::new();
        scene.play(leaf(1.0, 1.0));
        assert!(!scene.seal().into_evaluator(120.0).has_audio());
    }

    #[test]
    fn container_duration_override_rescales_sound() {
        // The inner sequence's 2s of content (the sound itself) is squeezed
        // into a 1s window: the clip is consumed twice as fast and stays
        // audible exactly for the squeezed span.
        let inner = seq![Sound::new(tone(2.0))];
        let mut scene = RanimScene::new();
        scene.play(inner.with_duration(1.0));
        let (_, start, end) = audible_region(scene);
        assert!((start - 0.0).abs() < 0.01);
        assert!((end - 1.0).abs() < 0.01);
    }

    #[test]
    fn container_rate_func_warps_the_sound_window() {
        // ease_in_quad maps scene progress u to content u²; the sound occupies
        // content [1, 2] of 2, so it becomes audible when u² >= 1/2, at scene
        // time 2·sqrt(1/2) ≈ 1.4142.
        let inner = seq![leaf(1.0, 1.0), Sound::new(tone(1.0))];
        let mut scene = RanimScene::new();
        scene.play(inner.with_rate_func(ease_in_quad));

        let (_, start, end) = audible_region(scene);
        let expected_start = 2.0 * (0.5f64).sqrt();
        assert!((start - expected_start).abs() < 0.01, "start {start}");
        assert!((end - 2.0).abs() < 0.01, "end {end}");
    }

    #[test]
    fn sound_rate_func_warps_its_content_progress() {
        // A linear ramp clip under ease_in_quad: scene time t reads clip
        // position t², so amplitude at t=0.5 is 0.25 (not 0.5).
        let clip = AudioClip::from_pcm(
            (0..48_000).map(|i| i as f32 / 48_000.0).collect::<Vec<_>>(),
            48_000,
            1,
        );
        let mut scene = RanimScene::new();
        scene.play(Sound::new(clip).with_rate_func(ease_in_quad));
        let evaluator = scene.seal().into_evaluator(120.0);
        let buf = evaluator.mix_audio(1.0, 48_000);
        // Stereo-interleaved: the L sample of frame (0.5 s × 48 kHz).
        let mid = buf[(0.5 * 48_000.0) as usize * 2];
        assert!((mid - 0.25).abs() < 0.01, "amplitude at 0.5s was {mid}");
    }

    #[test]
    fn warped_container_of_warped_sounds_mixes_all_of_them() {
        // Non-linear rates everywhere: nothing is pre-bakeable, so the whole
        // stack must resolve through the per-sample walk — every leaf, no
        // marking. 10 overlapping constant tones of 0.25 must sum to ~2.5.
        let clip = AudioClip::from_pcm(vec![0.25f32; 48_000], 48_000, 1);
        let mut stack = AnimStack::new();
        for _ in 0..10 {
            stack.push(Sound::new(clip.clone()).with_rate_func(ease_in_quad));
        }
        let mut scene = RanimScene::new();
        scene.play(stack.with_rate_func(ease_in_quad));
        let buf = scene.seal().into_evaluator(120.0).mix_audio(0.5, 48_000);
        for frame in [0usize, 12_000, 23_999] {
            assert!(
                (buf[frame * 2] - 2.5).abs() < 1e-3,
                "frame {frame}: {} (expected 10 × 0.25)",
                buf[frame * 2]
            );
        }
    }

    #[test]
    fn nested_containers_compose_the_sound_window() {
        let inner = seq![leaf(1.0, 1.0), Sound::new(tone(1.0))];
        let mut scene = RanimScene::new();
        scene.play(seq![inner, Sound::new(tone(0.5))]);
        let evaluator = scene.seal().into_evaluator(120.0);
        let buf = evaluator.mix_audio(evaluator.total_secs(), MASTER_SAMPLE_RATE);
        let at = |sec: f64| buf[(sec * MASTER_SAMPLE_RATE as f64) as usize * 2];
        // The first sound plays over [1, 2] (inside the inner sequence), the
        // second over [2, 2.5] (after it in the outer sequence).
        assert!(at(0.5).abs() < 1e-4);
        assert!(at(1.5).abs() > 1e-4);
        assert!(at(2.25).abs() > 1e-4);
    }

    #[test]
    fn sound_with_a_tail_extends_the_scene_to_its_window() {
        // Placement semantics are uniform: a sound's window occupies
        // timeline space just like a visual's. Clip sample counts are
        // quantized, so an author synthesizing to a target duration should
        // floor (not ceil) the sample count to avoid a sub-frame tail.
        let clip_len = 48_001; // 1.0000208..s at 48 kHz
        let clip = AudioClip::from_pcm(vec![0.5f32; clip_len], 48_000, 1);
        let mut scene = RanimScene::new();
        scene.play(stack![
            Static(VItem::default()).with_duration(1.0),
            Sound::new(clip)
        ]);
        assert!((scene.seal().total_secs() - clip_len as f64 / 48_000.0).abs() < 1e-9);
    }

    #[test]
    fn linear_rate_is_structural() {
        // A default-built cell must carry the structural linear rate
        // (`None`), never an identity fn pointer — function pointer
        // addresses are not guaranteed unique, so linearity cannot be
        // detected by comparison. Future mixing fast paths compose affine
        // maps only while every rate along the path is linear.
        let cells = [
            AnimSequence::new().build(),
            AnimStack::new().build(),
            Sound::new(tone(0.01)).build(),
            leaf(1.0, 1.0).build(),
        ];
        for cell in &cells {
            assert!(
                cell.rate_func.is_none(),
                "a default-built cell must carry the structural linear rate"
            );
        }
        let warped = leaf(1.0, 1.0).with_rate_func(ease_in_quad).build();
        assert!(warped.rate_func.is_some());
    }

    #[test]
    fn baked_audio_matches_point_semantics() {
        // One scene exercising every mixing shape: sequential and
        // overlapping sounds, a duration override, container and leaf rate
        // warps, fades, gain, speed, a stereo clip, and visual filler cells.
        // The seal-time bake must equal a fresh per-sample walk of the same
        // tree (the point-semantics spec).
        let ramp = |i: usize| 0.4 * i as f32 / 48_000.0;
        let stereo = AudioClip::from_pcm(
            (0..48_000)
                .flat_map(|i| [ramp(i), ramp(i)])
                .collect::<Vec<_>>(),
            48_000,
            2,
        );
        let warped = seq![
            Static(VItem::default()).with_duration(1.0),
            Sound::new(tone(1.0))
        ]
        .with_rate_func(ease_in_quad);

        let mut scene = RanimScene::new();
        scene.play(stack![
            seq![
                Sound::new(AudioClip::sine(440.0, 1.0, 0.5))
                    .with_fade_in(0.25)
                    .with_gain(0.8),
                Sound::new(stereo).with_speed(2.0),
            ],
            Sound::new(AudioClip::sine(880.0, 2.0, 0.3)).at(0.5),
            seq![Sound::new(tone(2.0))].with_duration(1.7),
            warped,
            Sound::new(tone(1.0)).with_rate_func(ease_in_quad),
            Static(VItem::default()).with_duration(4.0),
        ]);
        let sealed = scene.seal();
        let baked = sealed.audio().clone();
        let evaluator = sealed.into_evaluator(120.0);
        assert_eq!(baked, evaluator.audio().clone());

        fn cell_at(cell: &AnimNode, x: f64) -> [f32; 2] {
            if !cell.enabled || x < cell.time_range.start || x >= cell.time_range.end {
                return [0.0; 2];
            }
            let raw = (x - cell.time_range.start) / cell.duration_secs();
            let own = cell.internal_time_secs * cell.rate_func.map_or(raw, |rate| rate(raw));
            match &cell.content {
                NodeContent::Audio(track) => track.sample_at(own),
                _ => {
                    let mut acc = [0.0f32; 2];
                    for child in cell.children() {
                        let [l, r] = cell_at(child, own);
                        acc[0] += l;
                        acc[1] += r;
                    }
                    acc
                }
            }
        }

        let frames = baked.len() / 2;
        for frame in 0..frames {
            let x = frame as f64 / MASTER_SAMPLE_RATE as f64;
            let mut acc = [0.0f32; 2];
            for cell in evaluator.cells() {
                let [l, r] = cell_at(cell, x);
                acc[0] += l;
                acc[1] += r;
            }
            assert!(
                (baked[frame * 2] - acc[0]).abs() < 1e-6
                    && (baked[frame * 2 + 1] - acc[1]).abs() < 1e-6,
                "frame {frame}: baked [{}, {}] vs walk {acc:?}",
                baked[frame * 2],
                baked[frame * 2 + 1]
            );
        }
    }
}
