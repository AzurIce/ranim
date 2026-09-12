//! Pure animation evaluation and hierarchical type-erased composition.
//!
//! The runtime tree is a closed world of node content ([`NodeContent`](crate::animation::node::NodeContent)): the
//! time-combinator vocabulary (sequence, stack) plus the leaf forms. This
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
        node::{AnimNode, AnimationInfoKind, NodeContent},
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
    fn containers_lower_children_to_expected_timelines() {
        // Default lowering: linear rate, one second.
        let animation = Static(VItem::default()).into_anim_node();
        assert_eq!(animation.time_range(), 0.0..1.0);
        let animation = Static(VItem::default()).with_duration(2.0).into_anim_node();
        assert_eq!(animation.time_range(), 0.0..2.0);

        // Sequence concatenates child durations; stack keeps them at the
        // same origin, honouring explicit child positions.
        let sequence = seq![leaf(1.0, 2.0), leaf(2.0, 3.0)];
        assert_eq!(sequence.built_animations()[0].time_range(), 0.0..2.0);
        assert_eq!(sequence.built_animations()[1].time_range(), 2.0..5.0);
        let stack = stack![leaf(1.0, 2.0), leaf(2.0, 3.0).at(1.0)];
        assert_eq!(stack.duration_secs(), 4.0);
        assert_eq!(stack.built_animations()[0].time_range(), 0.0..2.0);
        assert_eq!(stack.built_animations()[1].time_range(), 1.0..4.0);

        let mut dynamic = AnimStack::new();
        dynamic.push(leaf(1.0, 1.0)).push(leaf(2.0, 3.0));
        assert_eq!(dynamic.duration_secs(), 3.0);
        assert_eq!(dynamic.built_animations()[0].time_range(), 0.0..1.0);
        assert_eq!(dynamic.built_animations()[1].time_range(), 0.0..3.0);

        // Collectors build the same containers.
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
    }

    #[test]
    fn offsets_shift_the_whole_container_timeline() {
        let animation = leaf(1.0, 2.0).at(3.0).into_anim_node();
        assert_eq!(animation.time_range(), 3.0..5.0);

        let sequence = seq![leaf(1.0, 2.0), leaf(2.0, 3.0)];
        assert_eq!(sequence.at(5.0).into_anim_node().time_range(), 5.0..10.0);

        let stack = stack![leaf(1.0, 2.0), leaf(2.0, 3.0).at(1.0)];
        assert_eq!(stack.at(10.0).into_anim_node().time_range(), 10.0..14.0);

        // An extended sequence keeps its local gaps after being repositioned.
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
    fn holds_sample_active_items_and_repeat_without_nesting() {
        // A hold samples only animations active before the cursor.
        let mut sequence = AnimSequence::new();
        sequence
            .push(stack![leaf(1.0, 1.0), leaf(2.0, 2.0)])
            .hold(1.0);
        assert_eq!(sequence.built_animations().len(), 2);
        assert_eq!(sampled_xs(&sequence.built_animations()[1], 2.5), vec![2.0]);

        // Repeated holds become adjacent static cells.
        let mut sequence = AnimSequence::new();
        sequence.push(leaf(3.0, 1.0)).hold(1.0).hold(2.0);
        assert_eq!(sequence.cursor_sec(), 4.0);
        assert_eq!(sequence.built_animations().len(), 3);
        assert_eq!(sequence.built_animations()[1].time_range(), 1.0..2.0);
        assert_eq!(sequence.built_animations()[2].time_range(), 2.0..4.0);
        assert_eq!(sampled_xs(&sequence.built_animations()[2], 3.5), vec![3.0]);

        // Every replay flattens the dyn batch instead of nesting it.
        let mut sequence = AnimSequence::new();
        sequence
            .push(stack![leaf(1.0, 1.0), leaf(2.0, 1.0)])
            .hold(1.0)
            .hold(1.0);
        let mut first = Vec::new();
        sequence.built_animations()[1].eval_at(1.5, &mut first);
        let mut second = Vec::new();
        sequence.built_animations()[2].eval_at(2.5, &mut second);
        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 2);
        assert_eq!(evaluated_xs(second), vec![1.0, 2.0]);
    }

    #[test]
    fn holds_use_final_evaluations_and_forward_does_not_hold() {
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

        // `forward` only advances the cursor; it does not emit a hold cell.
        let mut sequence = AnimSequence::new();
        sequence.push(leaf(4.0, 1.0)).forward(1.0).hold(1.0);
        assert_eq!(sequence.cursor_sec(), 3.0);
        assert_eq!(sequence.built_animations().len(), 1);

        // Nested sequences keep their own final evaluation.
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
        assert_eq!(animation.time_range(), 0.0..1.5);
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

    #[test]
    fn lagged_fill_structure_is_materialized_as_per_item_tracks() {
        let animation = lagged![
            0.5;
            progress_leaf(0.0).with_duration(1.0),
            progress_leaf(10.0).with_duration(1.0),
        ]
        .into_anim_node();
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
    fn sound_windows_follow_placement() {
        // Sequential placement occupies exactly the sound's own window.
        let mut scene = RanimScene::new();
        scene.play(seq![leaf(1.0, 1.0), Sound::new(tone(2.0))]);
        let (total, start, end) = audible_region(scene);
        assert!((total - 3.0).abs() < 1e-9);
        assert!((start - 1.0).abs() < 0.01);
        assert!((end - 3.0).abs() < 0.01);

        // `.at()` shifts that window on the timeline.
        let mut scene = RanimScene::new();
        scene.play(stack![Sound::new(tone(1.0)).at(2.0)]);
        let (_, start, end) = audible_region(scene);
        assert!((start - 2.0).abs() < 0.01);
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
