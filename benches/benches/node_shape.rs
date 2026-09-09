//! Spike: dyn-container tree (today's shape) vs enum-node tree (proposed).
//!
//! Two semantics-identical toy trees:
//!
//! - `dyn` — mirrors today: each node is a shell holding
//!   `Box<dyn Container>`; the concrete containers (Seq/Stack/leaf) privately
//!   own their children, and everything navigates through trait vtables
//!   (`children()`, `internal_time()`, `child_infos()`).
//! - `enum` — mirrors the proposal: a uniform `Node { kind, children,
//!   internal, .. }` with children on the node and per-kind logic in match
//!   arms; no container objects exist at runtime.
//!
//! Three consumer paths per design:
//!
//! - `eval`  — visual eval, one `eval_at` per frame (60 fps × 5 s);
//! - `walk`  — per-sample audio walk (240k samples), the residual/bake
//!   navigation path;
//! - `info`  — the introspection tree (`AnimationInfo` equivalent).
//!
//! Correctness cross-check: both designs must agree on eval outputs, audio
//! samples, and info trees before benchmarking.

use std::f64::consts::TAU;
use std::hint::black_box;
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, SamplingMode, criterion_group, criterion_main};

const SR: f64 = 48_000.0;
const TOTAL: f64 = 5.0;
const FRAMES: usize = (TOTAL * SR) as usize;
const EVAL_FRAMES: usize = 300; // 5 s at 60 fps

// MARK: shared bits

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Seq,
    Stack,
    Leaf,
    Audio,
}

#[derive(PartialEq, Debug)]
struct Info {
    kind: Kind,
    range: (f64, f64),
    internal: f64,
    children: Vec<Info>,
}

struct Track {
    pcm: Arc<[f32]>, // mono at SR
    len: f64,        // audible content seconds
}

impl Track {
    fn sample_at(&self, own: f64) -> f32 {
        if !(0.0..self.len).contains(&own) {
            return 0.0;
        }
        let src = own * SR;
        let f0 = src.floor() as usize;
        if f0 >= self.pcm.len() {
            return 0.0;
        }
        let frac = (src - f0 as f64) as f32;
        let f1 = (f0 + 1).min(self.pcm.len() - 1);
        self.pcm[f0] + (self.pcm[f1] - self.pcm[f0]) * frac
    }
}

fn sine_pcm() -> Arc<[f32]> {
    (0..FRAMES)
        .map(|i| (0.5 * (TAU * 440.0 * i as f64 / SR).sin()) as f32)
        .collect()
}

// MARK: design A — dyn containers (today's shape)

mod dyn_design {
    use super::{Info, Kind, Track};

    pub struct Node {
        inner: Box<dyn Container>,
        rate: Option<fn(f64) -> f64>,
        start: f64,
        end: f64,
        enabled: bool,
    }

    pub trait Container {
        fn kind(&self) -> Kind;
        fn eval_into(&self, alpha: f64, out: &mut Vec<f32>);
        fn internal_time(&self) -> f64 {
            1.0
        }
        fn children(&self) -> &[Node] {
            &[]
        }
        fn child_infos(&self) -> Vec<Info> {
            Vec::new()
        }
        fn sample_at(&self, _own: f64) -> f32 {
            0.0
        }
    }

    struct Seq {
        animations: Vec<Node>,
        cursor_sec: f64,
    }

    impl Container for Seq {
        fn kind(&self) -> Kind {
            Kind::Seq
        }
        fn eval_into(&self, alpha: f64, out: &mut Vec<f32>) {
            let content = self.cursor_sec * alpha;
            if let Some(child) = self
                .animations
                .iter()
                .rev()
                .find(|c| content >= c.start && content < c.end)
            {
                child.eval_at(content, out);
            }
        }
        fn internal_time(&self) -> f64 {
            self.cursor_sec
        }
        fn children(&self) -> &[Node] {
            &self.animations
        }
        fn child_infos(&self) -> Vec<Info> {
            self.animations.iter().map(|c| c.info()).collect()
        }
    }

    struct Stack {
        animations: Vec<Node>,
        duration: f64,
    }

    impl Container for Stack {
        fn kind(&self) -> Kind {
            Kind::Stack
        }
        fn eval_into(&self, alpha: f64, out: &mut Vec<f32>) {
            let content = self.duration * alpha;
            for child in &self.animations {
                if content >= child.start && content < child.end {
                    child.eval_at(content, out);
                }
            }
        }
        fn internal_time(&self) -> f64 {
            self.duration
        }
        fn children(&self) -> &[Node] {
            &self.animations
        }
        fn child_infos(&self) -> Vec<Info> {
            self.animations.iter().map(|c| c.info()).collect()
        }
    }

    struct VisualLeaf;

    impl Container for VisualLeaf {
        fn kind(&self) -> Kind {
            Kind::Leaf
        }
        fn eval_into(&self, alpha: f64, out: &mut Vec<f32>) {
            out.push(alpha as f32);
        }
    }

    struct AudioLeaf(Track);

    impl Container for AudioLeaf {
        fn kind(&self) -> Kind {
            Kind::Audio
        }
        fn eval_into(&self, _alpha: f64, _out: &mut Vec<f32>) {}
        fn internal_time(&self) -> f64 {
            self.0.len
        }
        fn sample_at(&self, own: f64) -> f32 {
            self.0.sample_at(own)
        }
    }

    impl Node {
        #[allow(clippy::too_many_arguments)]
        fn new(inner: Box<dyn Container>, start: f64, end: f64) -> Self {
            Self {
                inner,
                rate: None,
                start,
                end,
                enabled: true,
            }
        }

        pub fn seq(children: Vec<Node>) -> Self {
            let cursor = children.last().map(|c| c.end).unwrap_or(0.0);
            Self::new(
                Box::new(Seq {
                    animations: children,
                    cursor_sec: cursor,
                }),
                0.0,
                cursor,
            )
        }

        pub fn stack(children: Vec<Node>) -> Self {
            let duration = children.iter().map(|c| c.end).fold(0.0_f64, f64::max);
            Self::new(
                Box::new(Stack {
                    animations: children,
                    duration,
                }),
                0.0,
                duration,
            )
        }

        pub fn visual(start: f64, end: f64) -> Self {
            Self::new(Box::new(VisualLeaf), start, end)
        }

        pub fn audio(start: f64, end: f64, track: &Track) -> Self {
            let len = end - start;
            Self::new(
                Box::new(AudioLeaf(Track {
                    pcm: track.pcm.clone(),
                    len,
                })),
                start,
                end,
            )
        }

        pub fn eval_at(&self, sec: f64, out: &mut Vec<f32>) {
            if !self.enabled || sec < self.start || sec >= self.end {
                return;
            }
            let raw = (sec - self.start) / (self.end - self.start);
            let alpha = self.rate.map_or(raw, |rate| rate(raw));
            self.inner.eval_into(alpha, out);
        }

        pub fn sample_at(&self, x: f64) -> f32 {
            if !self.enabled || x < self.start || x >= self.end {
                return 0.0;
            }
            let raw = (x - self.start) / (self.end - self.start);
            let own = self.inner.internal_time() * self.rate.map_or(raw, |rate| rate(raw));
            if self.inner.kind() == Kind::Audio {
                return self.inner.sample_at(own);
            }
            let mut acc = 0.0;
            for child in self.inner.children() {
                acc += child.sample_at(own);
            }
            acc
        }

        pub fn info(&self) -> Info {
            Info {
                kind: self.inner.kind(),
                range: (self.start, self.end),
                internal: self.inner.internal_time(),
                children: self.inner.child_infos(),
            }
        }
    }
}

// MARK: design B — enum nodes (proposed shape)

mod enum_design {
    use super::{Info, Kind, Track};

    enum Inner {
        Seq,
        Stack,
        Leaf,
        Audio(Track),
    }

    pub struct Node {
        kind: Inner,
        children: Vec<Node>,
        internal: f64,
        rate: Option<fn(f64) -> f64>,
        start: f64,
        end: f64,
        enabled: bool,
    }

    impl Node {
        fn kind(&self) -> Kind {
            match &self.kind {
                Inner::Seq => Kind::Seq,
                Inner::Stack => Kind::Stack,
                Inner::Leaf => Kind::Leaf,
                Inner::Audio(_) => Kind::Audio,
            }
        }

        fn shell(kind: Inner, children: Vec<Node>, internal: f64, start: f64, end: f64) -> Self {
            Self {
                kind,
                children,
                internal,
                rate: None,
                start,
                end,
                enabled: true,
            }
        }

        pub fn seq(children: Vec<Node>) -> Self {
            let cursor = children.last().map(|c| c.end).unwrap_or(0.0);
            Self::shell(Inner::Seq, children, cursor, 0.0, cursor)
        }

        pub fn stack(children: Vec<Node>) -> Self {
            let duration = children.iter().map(|c| c.end).fold(0.0_f64, f64::max);
            Self::shell(Inner::Stack, children, duration, 0.0, duration)
        }

        pub fn visual(start: f64, end: f64) -> Self {
            // A bare visual leaf has no content axis (the dyn default is 1.0).
            Self::shell(Inner::Leaf, Vec::new(), 1.0, start, end)
        }

        pub fn audio(start: f64, end: f64, track: &Track) -> Self {
            let len = end - start;
            Self::shell(
                Inner::Audio(Track {
                    pcm: track.pcm.clone(),
                    len,
                }),
                Vec::new(),
                len,
                start,
                end,
            )
        }

        pub fn eval_at(&self, sec: f64, out: &mut Vec<f32>) {
            if !self.enabled || sec < self.start || sec >= self.end {
                return;
            }
            let raw = (sec - self.start) / (self.end - self.start);
            let alpha = self.rate.map_or(raw, |rate| rate(raw));
            match &self.kind {
                Inner::Seq => {
                    let content = self.internal * alpha;
                    if let Some(child) = self
                        .children
                        .iter()
                        .rev()
                        .find(|c| content >= c.start && content < c.end)
                    {
                        child.eval_at(content, out);
                    }
                }
                Inner::Stack => {
                    let content = self.internal * alpha;
                    for child in &self.children {
                        if content >= child.start && content < child.end {
                            child.eval_at(content, out);
                        }
                    }
                }
                Inner::Leaf => out.push(alpha as f32),
                Inner::Audio(_) => {}
            }
        }

        pub fn sample_at(&self, x: f64) -> f32 {
            if !self.enabled || x < self.start || x >= self.end {
                return 0.0;
            }
            let raw = (x - self.start) / (self.end - self.start);
            let own = self.internal * self.rate.map_or(raw, |rate| rate(raw));
            match &self.kind {
                Inner::Audio(track) => track.sample_at(own),
                _ => {
                    let mut acc = 0.0;
                    for child in &self.children {
                        acc += child.sample_at(own);
                    }
                    acc
                }
            }
        }

        pub fn info(&self) -> Info {
            Info {
                kind: self.kind(),
                range: (self.start, self.end),
                internal: self.internal,
                children: self.children.iter().map(|c| c.info()).collect(),
            }
        }
    }
}

// MARK: scene builders (identical shapes for both designs)

trait NodeFactory: Sized {
    fn seq(children: Vec<Self>) -> Self;
    fn stack(children: Vec<Self>) -> Self;
    fn visual(start: f64, end: f64) -> Self;
    fn audio(start: f64, end: f64, track: &Track) -> Self;
}

impl NodeFactory for dyn_design::Node {
    fn seq(children: Vec<Self>) -> Self {
        Self::seq(children)
    }
    fn stack(children: Vec<Self>) -> Self {
        Self::stack(children)
    }
    fn visual(start: f64, end: f64) -> Self {
        Self::visual(start, end)
    }
    fn audio(start: f64, end: f64, track: &Track) -> Self {
        Self::audio(start, end, track)
    }
}

impl NodeFactory for enum_design::Node {
    fn seq(children: Vec<Self>) -> Self {
        Self::seq(children)
    }
    fn stack(children: Vec<Self>) -> Self {
        Self::stack(children)
    }
    fn visual(start: f64, end: f64) -> Self {
        Self::visual(start, end)
    }
    fn audio(start: f64, end: f64, track: &Track) -> Self {
        Self::audio(start, end, track)
    }
}

/// Sequential children filling the window (disjoint windows).
fn seq_scene<N: NodeFactory>(n: usize, audio: bool, track: &Track) -> N {
    let per = TOTAL / n as f64;
    let children = (0..n)
        .map(|i| {
            let start = i as f64 * per;
            if audio {
                N::audio(start, start + per, track)
            } else {
                N::visual(start, start + per)
            }
        })
        .collect();
    N::seq(children)
}

/// Overlapping children all covering the whole window.
fn stack_scene<N: NodeFactory>(n: usize, audio: bool, track: &Track) -> N {
    let children = (0..n)
        .map(|_| {
            if audio {
                N::audio(0.0, TOTAL, track)
            } else {
                N::visual(0.0, TOTAL)
            }
        })
        .collect();
    N::stack(children)
}

/// One child nested in `depth` stacked containers.
fn depth_scene<N: NodeFactory>(depth: usize, audio: bool, track: &Track) -> N {
    let mut cur = if audio {
        N::audio(0.0, TOTAL, track)
    } else {
        N::visual(0.0, TOTAL)
    };
    for _ in 1..depth {
        cur = N::stack(vec![cur]);
    }
    cur
}

// MARK: benchmark

/// The consumer-facing surface both designs expose (for generic benches).
trait BenchNode: Sized {
    fn eval_at(&self, sec: f64, out: &mut Vec<f32>);
    fn sample_at(&self, x: f64) -> f32;
    fn info(&self) -> Info;
}

impl BenchNode for dyn_design::Node {
    fn eval_at(&self, sec: f64, out: &mut Vec<f32>) {
        Self::eval_at(self, sec, out)
    }
    fn sample_at(&self, x: f64) -> f32 {
        Self::sample_at(self, x)
    }
    fn info(&self) -> Info {
        Self::info(self)
    }
}

impl BenchNode for enum_design::Node {
    fn eval_at(&self, sec: f64, out: &mut Vec<f32>) {
        Self::eval_at(self, sec, out)
    }
    fn sample_at(&self, x: f64) -> f32 {
        Self::sample_at(self, x)
    }
    fn info(&self) -> Info {
        Self::info(self)
    }
}

fn eval_frames<N: BenchNode>(root: &N) {
    let mut out = Vec::new();
    for f in 0..EVAL_FRAMES {
        let x = f as f64 * TOTAL / EVAL_FRAMES as f64;
        out.clear();
        root.eval_at(x, &mut out);
    }
    black_box(out);
}

fn walk_samples<N: BenchNode>(root: &N) -> f32 {
    let mut acc = 0.0f32;
    for frame in 0..FRAMES {
        acc += root.sample_at(frame as f64 / SR);
    }
    acc
}

fn node_shape_benchmark(c: &mut Criterion) {
    let pcm = sine_pcm();
    let track = Track { pcm, len: TOTAL };

    // Equivalence: both designs must produce identical results.
    {
        let dyn_seq = seq_scene::<dyn_design::Node>(100, false, &track);
        let enum_seq = seq_scene::<enum_design::Node>(100, false, &track);
        let mut a = Vec::new();
        let mut b = Vec::new();
        for f in 0..EVAL_FRAMES {
            a.clear();
            b.clear();
            let x = f as f64 * TOTAL / EVAL_FRAMES as f64;
            BenchNode::eval_at(&dyn_seq, x, &mut a);
            BenchNode::eval_at(&enum_seq, x, &mut b);
            if f % 50 == 0 {
                eprintln!("probe: eval frame {f}");
            }
            assert_eq!(a, b, "eval mismatch at {x}");
        }
        let dyn_audio = seq_scene::<dyn_design::Node>(100, true, &track);
        let enum_audio = seq_scene::<enum_design::Node>(100, true, &track);
        for f in 0..FRAMES {
            let x = f as f64 / SR;
            let da = dyn_audio.sample_at(x);
            let ea = enum_audio.sample_at(x);
            assert!((da - ea).abs() < 1e-6, "walk mismatch at {x}: {da} vs {ea}");
        }
        assert_eq!(dyn_seq.info(), enum_seq.info(), "info mismatch");
    }

    let mut group = c.benchmark_group("node_shape");
    group.sampling_mode(SamplingMode::Linear);
    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_secs(2));

    for (shape, n) in [("seq", 1000usize), ("stack", 100), ("depth", 64)] {
        let scenes: [(&str, dyn_design::Node, enum_design::Node); 2] = [
            (
                "visual",
                match shape {
                    "seq" => seq_scene(n, false, &track),
                    "stack" => stack_scene(n, false, &track),
                    _ => depth_scene(n, false, &track),
                },
                match shape {
                    "seq" => seq_scene(n, false, &track),
                    "stack" => stack_scene(n, false, &track),
                    _ => depth_scene(n, false, &track),
                },
            ),
            (
                "audio",
                match shape {
                    "seq" => seq_scene(n, true, &track),
                    "stack" => stack_scene(n, true, &track),
                    _ => depth_scene(n, true, &track),
                },
                match shape {
                    "seq" => seq_scene(n, true, &track),
                    "stack" => stack_scene(n, true, &track),
                    _ => depth_scene(n, true, &track),
                },
            ),
        ];

        // Visual eval (per frame) and info (whole tree) on the visual scenes.
        let [(_, dyn_vis, enum_vis), (_, dyn_aud, enum_aud)] = scenes;
        group.bench_function(BenchmarkId::new("dyn", format!("eval_{shape}_{n}")), |b| {
            b.iter(|| eval_frames::<dyn_design::Node>(black_box(&dyn_vis)))
        });
        group.bench_function(BenchmarkId::new("enum", format!("eval_{shape}_{n}")), |b| {
            b.iter(|| eval_frames::<enum_design::Node>(black_box(&enum_vis)))
        });
        group.bench_function(BenchmarkId::new("dyn", format!("info_{shape}_{n}")), |b| {
            b.iter(|| BenchNode::info(black_box(&dyn_vis)))
        });
        group.bench_function(BenchmarkId::new("enum", format!("info_{shape}_{n}")), |b| {
            b.iter(|| BenchNode::info(black_box(&enum_vis)))
        });
        // Per-sample audio walk on the audio scenes.
        group.bench_function(BenchmarkId::new("dyn", format!("walk_{shape}_{n}")), |b| {
            b.iter(|| walk_samples::<dyn_design::Node>(black_box(&dyn_aud)))
        });
        group.bench_function(BenchmarkId::new("enum", format!("walk_{shape}_{n}")), |b| {
            b.iter(|| walk_samples::<enum_design::Node>(black_box(&enum_aud)))
        });
    }

    group.finish();
}

criterion_group!(benches, node_shape_benchmark);
criterion_main!(benches);
