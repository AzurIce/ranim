//! Pure animation evaluation and hierarchical type-erased composition.
//!
//! The runtime tree is a closed world of node content ([`NodeContent`](crate::animation::node::NodeContent)): the
//! container children plus visual and audio leaf forms. This
//! module is split by layer:
//!
//! - [`node`] — the closed runtime core and all structural interpreters;
//! - [`eval`] — the open, typed visual-leaf protocol;
//! - [`build`] — the lowering protocol from authoring definitions to
//!   [`AnimNode`](crate::animation::node::AnimNode);
//! - [`compose`] — authoring sugar (`AnimSequence`, `AnimStack`, `AnimLagged`)
//!   that lowers to the core vocabulary.
//!
//! Authoring is open above the core: user leaves enter through
//! [`Eval`](crate::animation::eval::Eval), and combinators expressible as
//! placements (e.g. [`AnimLagged`](crate::animation::compose::lagged::AnimLagged))
//! desugar into the primitives in
//! [`IntoAnimNode`](crate::animation::build::IntoAnimNode).

/// Authoring protocol and playback wrappers.
pub mod build;
/// Built-in composition containers and iterator helpers.
pub mod compose;
/// Evaluation protocols and author-facing adapters.
pub mod eval;
/// Runtime animation nodes and their interpreters.
pub mod node;
/// Audio leaf animation: a sound placed and composed like any animation.
pub mod sound;

#[cfg(test)]
mod tests {
    use super::{
        build::{IntoAnimNode, PlaybackExt, Unplaced},
        compose::{AnimIterExt, lagged::LaggedFill, sequence::AnimSequence, stack::AnimStack},
        eval::{Eval, EvalExt, Static, StaticAnim, pure::Pure},
        node::{AnimNode, AnimationInfoKind, LeafContent, NodeContent},
    };
    use crate::{
        Extract,
        core_item::{CoreItem, DynItem, vitem::VItem},
        lagged, seq, stack,
    };

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

    fn leaf(x: f32, duration: f64) -> impl Unplaced {
        ShiftX(x).with_duration(duration)
    }

    fn progress_leaf(offset: f32) -> impl Unplaced {
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
    fn apply_alpha_to_writes_the_requested_progress_state() {
        let mut item = VItem::default();

        let animation =
            Pure::new(|alpha| ShiftAlpha(0.0).eval_alpha(alpha)).apply_alpha_to(&mut item, 0.25);
        assert_eq!(item.points[0].x, 0.25);

        let animation = animation.apply_to(&mut item);
        assert_eq!(item.points[0].x, 1.0);
        assert_eq!(animation.eval_alpha(0.5).points[0].x, 0.5);
    }

    #[test]
    fn containers_lower_children_to_expected_timelines() {
        let sequence = [leaf(1.0, 1.0), leaf(2.0, 2.0)].into_iter().into_seq();
        let stack = [leaf(1.0, 1.0), leaf(2.0, 2.0)].into_iter().into_stack();
        let lagged = [leaf(1.0, 1.0), leaf(2.0, 1.0)]
            .into_iter()
            .into_lagged(0.5);
        for (node, second_range) in [
            (sequence.into_anim_node(), 1.0..3.0),
            (stack.into_anim_node(), 0.0..2.0),
            (
                lagged
                    .with_leading(LaggedFill::Empty)
                    .with_trailing(LaggedFill::Empty)
                    .into_anim_node(),
                0.5..1.5,
            ),
        ] {
            assert!(matches!(node.content, NodeContent::Children(_)));
            // Lagged adds one container per child's track.
            let child = &node.children()[1];
            let range = child.children().first().unwrap_or(child).time_range();
            assert_eq!(range, second_range);
        }
        assert!(seq![].into_anim_node().children().is_empty());
        assert!(stack![].into_anim_node().children().is_empty());
    }

    #[test]
    fn runtime_children_evaluate_overlaps_in_order_and_keep_half_open_boundaries() {
        let animation = stack![
            progress_leaf(0.0).with_duration(2.0),
            progress_leaf(10.0).with_duration(2.0).at(1.0),
            leaf(100.0, 3.0).with_enabled(false),
        ]
        .into_anim_node();
        assert!(matches!(animation.content, NodeContent::Children(_)));
        assert_eq!(sampled_xs(&animation, 1.5), vec![0.75, 10.25]);
        assert_eq!(sampled_xs(&animation, 2.0), vec![10.5]);
        assert_eq!(sampled_xs(&animation, 3.0), vec![11.0]);
        assert_eq!(sampled_xs(&animation, 3.1), vec![]);
        let sequential = seq![leaf(1.0, 1.0), leaf(2.0, 1.0)].into_anim_node();
        assert!(matches!(sequential.content, NodeContent::Children(_)));
        assert_eq!(sampled_xs(&sequential, 1.0), vec![2.0]);
    }

    #[test]
    fn offsets_shift_the_whole_container_timeline() {
        let mut sequence = AnimSequence::new();
        sequence
            .push(leaf(1.0, 2.0))
            .forward(1.0)
            .push(leaf(2.0, 1.0));
        let info = sequence.at(10.0).into_anim_node().animation_info();
        assert_eq!(info.range, 10.0..14.0);
        assert_eq!(info.children[0].range, 0.0..2.0);
        assert_eq!(info.children[1].range, 3.0..4.0);
    }

    #[test]
    fn parametrized_sequence_remaps_the_group_timeline() {
        use crate::utils::rate_functions::ease_in_quad;

        let animation = seq![progress_leaf(0.0), progress_leaf(10.0)]
            .with_duration(4.0)
            .with_rate_func(ease_in_quad);
        let animation = animation.into_anim_node();

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
    fn extend_appends_direct_children_and_preserves_local_gaps() {
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

        let source = stack![leaf(2.0, 1.0), leaf(3.0, 2.0).at(1.0)];
        let mut stack = stack![leaf(1.0, 4.0)];
        stack.extend(source);
        assert_eq!(stack.duration_secs(), 4.0);
        assert_eq!(stack.built_animations().len(), 3);
        assert_eq!(stack.built_animations()[1].time_range(), 0.0..1.0);
        assert_eq!(stack.built_animations()[2].time_range(), 1.0..3.0);
    }

    #[test]
    fn holds_sample_active_items_and_repeat_without_nesting() {
        let mut sequence = AnimSequence::new();
        sequence
            .push(stack![leaf(1.0, 1.0), leaf(2.0, 2.0), leaf(3.0, 2.0)])
            .hold(1.0)
            .hold(2.0);
        assert_eq!(sequence.cursor_sec(), 5.0);
        for (index, sec) in [(1, 2.5), (2, 4.0)] {
            let held = &sequence.built_animations()[index];
            let mut items = Vec::new();
            held.eval_at(sec, &mut items);
            // Finished children disappear; consecutive holds keep a flat batch.
            assert_eq!(items.len(), 2);
            assert!(
                items
                    .iter()
                    .all(|item| (item.0.as_ref() as &dyn std::any::Any).is::<VItem>())
            );
            assert_eq!(evaluated_xs(items), vec![2.0, 3.0]);
            assert!(matches!(
                held.content,
                NodeContent::Leaf(LeafContent::Anim(_))
            ));
        }
    }

    #[test]
    fn holds_use_final_evaluations_and_forward_does_not_hold() {
        let mut shown = VItem::default();
        shown.points[0].x = 7.0;
        for (ending, expected) in [(shown.show(), vec![7.0]), (shown.hide(), vec![])] {
            let mut outer = AnimSequence::new();
            outer.push(seq![leaf(1.0, 1.0), ending]).hold(1.0);
            assert_eq!(sampled_xs(&outer.into_anim_node(), 1.5), expected);
        }
        let mut gap = AnimSequence::new();
        gap.push(leaf(4.0, 1.0)).forward(1.0).hold(1.0);
        assert!(sampled_xs(&gap.into_anim_node(), 2.5).is_empty());
    }

    #[test]
    fn sequence_lowering_removes_covered_points_after_holds_have_sampled_them() {
        let mut first = VItem::default();
        first.points[0].x = 2.0;
        let mut second = VItem::default();
        second.points[0].x = 3.0;
        let mut sequence = AnimSequence::new();
        sequence
            .push(progress_leaf(0.0))
            .push(first.show())
            .push(second.show())
            .hold(0.5)
            .push(second.hide())
            .hold(0.5);
        let animation = sequence.into_anim_node();
        assert_eq!(animation.children().len(), 2);
        assert_eq!(sampled_xs(&animation, 0.5), vec![0.5]);
        assert_eq!(sampled_xs(&animation, 1.0), vec![3.0]);
        assert_eq!(sampled_xs(&animation, 1.25), vec![3.0]);
        assert_eq!(sampled_xs(&animation, 1.5), vec![]);
        assert_eq!(sampled_xs(&animation, 2.0), vec![]);
    }

    #[test]
    fn sequence_final_points_replace_endpoints_without_affecting_stacks() {
        let mut shown = VItem::default();
        shown.points[0].x = 9.0;
        let sequence = seq![progress_leaf(0.0), shown.show(), shown.hide(), shown.show()]
            .with_duration(2.0)
            .at(3.0)
            .into_anim_node();
        assert_eq!(sampled_xs(&sequence, 4.0), vec![0.5]);
        assert_eq!(sampled_xs(&sequence, 5.0), vec![9.0]);
        assert_eq!(sequence.children().len(), 2);
        assert!(!sequence.children()[0].active_at(1.0));

        let hidden = seq![progress_leaf(0.0), shown.hide()].into_anim_node();
        assert_eq!(sampled_xs(&hidden, 0.5), vec![0.5]);
        assert!(sampled_xs(&hidden, 1.0).is_empty());
        let only_points = seq![shown.show(), shown.hide()].into_anim_node();
        assert_eq!(only_points.children().len(), 1);
        assert!(sampled_xs(&only_points, 0.0).is_empty());

        // Stacks retain all coincident endpoints, including point animations.
        let stack = stack![progress_leaf(0.0), shown.show().at(1.0)].into_anim_node();
        assert_eq!(sampled_xs(&stack, 1.0), vec![1.0, 9.0]);
    }

    #[test]
    fn lagged_staggers_and_fills_window_edges() {
        // Stagger by a ratio of each previous child's duration.
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

        // Default leading/trailing fills hold the edge states.
        let animation = lagged![
            0.5;
            progress_leaf(0.0).with_duration(1.0),
            progress_leaf(10.0).with_duration(1.0)
        ]
        .into_anim_node();
        let info = animation.animation_info();
        assert_eq!(info.kind, AnimationInfoKind::Children);
        let track_ranges: Vec<_> = info
            .children
            .iter()
            .map(|track| {
                track
                    .children
                    .iter()
                    .map(|child| child.range.clone())
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(
            track_ranges,
            [vec![0.0..1.0, 1.0..1.5], vec![0.0..0.5, 0.5..1.5]]
        );
        assert_eq!(sampled_xs(&animation, 0.25), vec![0.25, 10.0]);
        assert_eq!(sampled_xs(&animation, 1.0), vec![1.0, 10.5]);
        assert_eq!(sampled_xs(&animation, 1.5), vec![1.0, 11.0]);

        // `with_leading(Empty)` leaves the pre-window empty, trailing holds.
        let empty_leading = lagged![
            0.5;
            progress_leaf(0.0).with_duration(1.0),
            progress_leaf(10.0).with_duration(1.0)
        ]
        .with_leading(LaggedFill::Empty)
        .into_anim_node();
        assert_eq!(sampled_xs(&empty_leading, 0.25), vec![0.25]);
        assert_eq!(sampled_xs(&empty_leading, 1.0), vec![1.0, 10.5]);

        // A child ending with `hide` stays hidden after its own window.
        let mut shown = VItem::default();
        shown.points[0].x = 2.0;
        let hidden = lagged![0.5; seq![leaf(2.0, 1.0), shown.hide()]].into_anim_node();
        assert_eq!(sampled_xs(&hidden, 0.5), vec![2.0]);
        let mut items = Vec::new();
        hidden.eval_at(1.0, &mut items);
        assert!(items.is_empty());
    }

    // MARK: Sound leaves in the tree

    use super::sound::Sound;
    use crate::RanimScene;
    use crate::audio::{AudioClip, MASTER_SAMPLE_RATE};
    use crate::utils::rate_functions::ease_in_quad;

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
    fn sound_windows_follow_nested_placement_and_duration_overrides() {
        for (duration, expected_start, expected_end) in [(2.0, 2.0, 3.0), (1.0, 1.5, 2.0)] {
            let mut scene = RanimScene::new();
            scene.play(
                seq![leaf(1.0, 1.0), Sound::new(tone(1.0))]
                    .with_duration(duration)
                    .at(1.0),
            );
            let (total, start, end) = audible_region(scene);
            assert_eq!(total, expected_end);
            for (actual, expected) in [(start, expected_start), (end, expected_end)] {
                assert!((actual - expected).abs() < 0.01, "{actual} vs {expected}");
            }
        }
    }

    #[test]
    fn audio_leaves_are_silent_when_disabled_and_emit_no_visual_items() {
        let mut scene = RanimScene::new();
        scene.play(stack![Sound::new(tone(1.0)).with_enabled(false)]);
        let sealed = scene.seal();
        assert_eq!(sealed.eval_at_sec(0.5).count(), 0);
        let evaluator = sealed.into_evaluator(120.0);
        assert!(evaluator.has_audio());
        assert!(
            evaluator
                .mix_audio(1.0, MASTER_SAMPLE_RATE)
                .iter()
                .all(|s| s.abs() < 1e-4)
        );
        let mut visual = RanimScene::new();
        visual.play(leaf(1.0, 1.0));
        assert!(!visual.seal().into_evaluator(120.0).has_audio());
        let mut audio = RanimScene::new();
        audio.play(Sound::new(tone(0.01)));
        assert_eq!(audio.seal().eval_at_sec(0.005).count(), 0);
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
            AnimSequence::new().into_anim_node(),
            AnimStack::new().into_anim_node(),
            Sound::new(tone(0.01)).into_anim_node(),
            leaf(1.0, 1.0).into_anim_node(),
        ];
        for cell in &cells {
            assert!(
                cell.rate_func.is_none(),
                "a default-built cell must carry the structural linear rate"
            );
        }
        let warped = leaf(1.0, 1.0).with_rate_func(ease_in_quad).into_anim_node();
        assert!(warped.rate_func.is_some());
    }

    #[test]
    fn baked_audio_matches_point_semantics() {
        // One scene exercising every mixing shape: sequential and
        // overlapping sounds, duration overrides (one acting as a linear
        // speed change on a sound), container and leaf rate warps, fades,
        // gain, a stereo clip, and visual filler cells. The seal-time bake
        // must equal a fresh per-sample walk of the same tree (the
        // point-semantics spec).
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
                // A 2x speed change: half the window, pitch up an octave.
                Sound::new(stereo).with_duration(0.5),
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

        fn cell_at(cell: &AnimNode, x: f64) -> [f32; 2] {
            if !cell.enabled || x < cell.time_range.start || x >= cell.time_range.end {
                return [0.0; 2];
            }
            let raw = (x - cell.time_range.start) / cell.duration_secs();
            let own = cell.internal_time_secs * cell.rate_func.map_or(raw, |rate| rate(raw));
            match &cell.content {
                NodeContent::Leaf(LeafContent::Audio(track)) => track.sample_at(own),
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
