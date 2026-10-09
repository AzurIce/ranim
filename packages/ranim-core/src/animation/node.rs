//! Runtime animation nodes and their interpreters.
//!
//! This is the closed core of the animation tree: [`NodeContent`](crate::animation::node::NodeContent) is the
//! runtime vocabulary (children or a leaf), [`AnimNode`](crate::animation::node::AnimNode)
//! wraps it with the timing shell, and all structural traversals — visual
//! evaluation, seal-time audio baking, and preview introspection — live here.
//!
//! Authoring sugar (`AnimSequence`, `AnimStack`, `AnimLagged`) is layered on
//! top in [`crate::animation::compose`]; user leaf protocols live in
//! [`crate::animation::eval`].

use std::ops::Range;

use crate::{audio::AudioTrack, core_item::DynItem, utils::rate_functions::linear};

use super::eval::{EvalDyn, Static};

/// Runtime animation content category used by preview tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationInfoKind {
    /// A typed evaluator without animation children.
    Anim,
    /// A container of animations placed in its local content coordinates.
    Children,
    /// An audio leaf: resolved into the audio plane at seal time.
    Audio,
}

/// Hierarchical runtime animation information used by preview tooling.
#[derive(Clone)]
pub struct AnimationInfo {
    /// Node display name, normally its concrete evaluator or container type name.
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

/// Runtime structure: either placed children or one leaf payload.
///
/// Sequential, parallel, and staggered placement belong to authoring
/// combinators. Every container evaluates all children containing its content
/// time, in insertion order.
pub(in crate::animation) enum NodeContent {
    /// Animations placed in this node's local content coordinates.
    Children(Vec<AnimNode>),
    /// A visual or audio leaf without animation children.
    Leaf(LeafContent),
}

/// The payloads interpreted at the leaves of the runtime tree.
pub(in crate::animation) enum LeafContent {
    /// A typed evaluator (`Eval` implementation) — no children.
    Anim(Box<dyn EvalDyn>),
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
            NodeContent::Children(_) => AnimationInfoKind::Children,
            NodeContent::Leaf(LeafContent::Anim(_)) => AnimationInfoKind::Anim,
            NodeContent::Leaf(LeafContent::Audio(_)) => AnimationInfoKind::Audio,
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
    /// for bare visual leaves, `0.0` for captured constant batches.
    pub(in crate::animation) internal_time_secs: f64,
    pub(in crate::animation) rate_func: Option<fn(f64) -> f64>,
    pub(in crate::animation) time_range: Range<f64>,
    /// Whether the final parent boundary may include this node's right endpoint.
    /// Sequence lowering closes it when a later point animation wins.
    pub(in crate::animation) end_inclusive: bool,
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
    pub fn children(&self) -> &[AnimNode] {
        match &self.content {
            NodeContent::Children(children) => children,
            _ => &[],
        }
    }

    /// Mutable access to placed children; leaves return an empty slice.
    pub fn children_mut(&mut self) -> &mut [AnimNode] {
        match &mut self.content {
            NodeContent::Children(children) => children,
            _ => &mut [],
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

    /// Display name, normally the concrete evaluator type name captured before erasure.
    pub fn anim_name(&self) -> &str {
        self.anim_name
    }

    /// Whether the given scene time is inside this clip's window, including
    /// the right endpoint unless it was excluded during sequence lowering.
    pub fn active_at(&self, sec: f64) -> bool {
        sec >= self.time_range.start
            && (sec < self.time_range.end || (self.end_inclusive && sec == self.time_range.end))
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
            NodeContent::Children(children) => {
                let content = self.internal_time_secs * alpha;
                eval_children(children, content, self.internal_time_secs, output)
            }
            NodeContent::Leaf(LeafContent::Anim(eval)) => eval.eval_into(alpha, output),
            NodeContent::Leaf(LeafContent::Audio(_)) => {}
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
            NodeContent::Leaf(LeafContent::Audio(track)) => {
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
            NodeContent::Leaf(LeafContent::Audio(_)) => true,
            _ => self.children().iter().any(AnimNode::has_audio),
        }
    }

    pub(in crate::animation) fn contains_sec(&self, sec: f64, parent_duration: f64) -> bool {
        self.time_range.contains(&sec)
            || (self.end_inclusive && sec == parent_duration && sec == self.time_range.end)
    }

    /// Describe this node and its descendants for inspection tools.
    pub fn animation_info(&self) -> AnimationInfo {
        AnimationInfo {
            anim_name: self.anim_name.to_string(),
            kind: self.content.info_kind(),
            range: self.time_range.clone(),
            content_duration_secs: self.internal_time_secs,
            rate_func: self.rate_func.unwrap_or(linear),
            enabled: self.enabled,
            sim_step: match &self.content {
                NodeContent::Leaf(LeafContent::Anim(eval)) => eval.sim_step(),
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

/// Evaluate every child containing the content time, preserving output order.
fn eval_children(children: &[AnimNode], content_sec: f64, extent: f64, output: &mut Vec<DynItem>) {
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
                NodeContent::Leaf(LeafContent::Audio(track)) => Residual::Leaf {
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

/// Build a constant animation replaying sampled items over `time_range`.
pub(in crate::animation) fn constant_node(state: Vec<DynItem>, time_range: Range<f64>) -> AnimNode {
    AnimNode {
        content: NodeContent::Leaf(LeafContent::Anim(Box::new(Static(state)))),
        internal_time_secs: 0.0,
        rate_func: None,
        time_range,
        end_inclusive: true,
        enabled: true,
        anim_name: std::any::type_name::<Static<Vec<DynItem>>>(),
    }
}
