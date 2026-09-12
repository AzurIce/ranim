//! Scrolling piano-roll visualization of the bundled Nyan Cat MIDI file,
//! with the song synthesized into the scene as a [`Sound`] leaf.
//!
//! MIDI source: <https://freemidi.org/getter-25332>
//! See `SOURCE.md` for the download filename, retrieval date, and checksum.

pub mod midi;
pub mod visual;

use std::{f64::consts::TAU, sync::Arc};

use midi::parse_song;
use ranim::{glam::DVec3, prelude::*, utils::rate_functions::linear};
use ranim_core::animation::{
    build::{IntoAnimNode, Unplaced},
    eval::StaticAnim,
};
use visual::{
    FRAME_HEIGHT, HitEffectsEval, MidiNotesEval, PianoKeyboardEval, PianoLayout, SingleNoteEval,
    make_background,
};

/// Synthesize the whole song into one clip: each note becomes a harmonic
/// tone with a short attack/release envelope; notes are summed and the
/// result is peak-normalized.
///
/// Thousands of dense note events belong in one pre-summed clip — a
/// [`Sound`] leaf per note would scan the scene window once per leaf during
/// mixing. Leaves are for sparse placement, not for event tracks.
fn render_song_audio(song: &midi::MidiSong) -> AudioClip {
    const SAMPLE_RATE: u32 = 48_000;
    const ATTACK_SECS: f64 = 0.004;
    const RELEASE_SECS: f64 = 0.04;
    // The parser closes stuck notes at song end; cap the drone.
    const MAX_NOTE_SECS: f64 = 5.0;

    // Floor, not ceil: the clip must not run past the scene's visual window,
    // or the sub-frame tail renders without a camera (D0002).
    let total = (song.duration_secs * f64::from(SAMPLE_RATE)) as usize;
    let mut pcm = vec![0.0_f64; total];
    for note in &song.notes {
        let freq = 440.0 * ((f64::from(note.key) - 69.0) / 12.0).exp2();
        let amp = f64::from(note.velocity) / 127.0;
        let note_len = (note.end_sec - note.start_sec).min(MAX_NOTE_SECS);
        let len = (note_len * f64::from(SAMPLE_RATE)).ceil() as usize;
        let start = (note.start_sec * f64::from(SAMPLE_RATE)).round() as usize;
        let end = (start + len).min(total);
        let attack = ((ATTACK_SECS * f64::from(SAMPLE_RATE)) as usize).max(1);
        let release = ((RELEASE_SECS * f64::from(SAMPLE_RATE)) as usize).max(1);
        for (j, sample) in pcm[start..end].iter_mut().enumerate() {
            let mut envelope = amp;
            if j < attack {
                envelope *= j as f64 / attack as f64;
            }
            if j > len.saturating_sub(release) {
                envelope *= (len - j) as f64 / release as f64;
            }
            let t = j as f64 / f64::from(SAMPLE_RATE);
            let wave = 0.6 * (TAU * freq * t).sin()
                + 0.25 * (TAU * 2.0 * freq * t).sin()
                + 0.15 * (TAU * 3.0 * freq * t).sin();
            *sample += envelope * wave;
        }
    }

    let peak = pcm
        .iter()
        .fold(0.0_f64, |max, sample| max.max(sample.abs()));
    let scale = if peak > 0.8 { 0.8 / peak } else { 1.0 };
    AudioClip::from_pcm(
        pcm.into_iter()
            .map(|sample| (sample * scale) as f32)
            .collect::<Vec<_>>(),
        SAMPLE_RATE,
        1,
    )
}

fn common_layers(song: &Arc<midi::MidiSong>, layout: PianoLayout) -> AnimStack {
    let duration = song.duration_secs;
    let background = make_background(layout);

    stack![
        background.show().with_duration(duration),
        PianoKeyboardEval {
            song: song.clone(),
            layout,
        }
        .with_duration(duration)
        .with_rate_func(linear),
        HitEffectsEval {
            song: song.clone(),
            layout,
        }
        .with_duration(duration)
        .with_rate_func(linear),
    ]
}

fn camera(duration: f64) -> impl IntoAnimNode {
    CameraFrame {
        pos: DVec3::ZERO,
        frame_height: FRAME_HEIGHT,
        ..Default::default()
    }
    .show()
    .with_duration(duration)
}

#[scene(clear_color = "#070711")]
#[wasm_demo_doc]
#[output(fps = 30, dir = "./output/midi_visualizer")]
fn midi_visualizer(r: &mut RanimScene) {
    let song = Arc::new(parse_song());
    let layout = PianoLayout::new(&song);
    let duration = song.duration_secs;

    r.play(camera(duration));
    r.play(Sound::new(render_song_audio(&song)));
    r.play(stack![
        MidiNotesEval {
            song: song.clone(),
            layout,
        }
        .with_duration(duration)
        .with_rate_func(linear),
        common_layers(&song, layout),
    ]);
    r.insert_time_mark(duration * 0.5, TimeMark::Capture("preview.png".to_string()));
}

#[scene(clear_color = "#070711")]
#[wasm_demo_doc]
#[output(fps = 30, dir = "./output/midi_visualizer")]
fn midi_visualizer_per_note(r: &mut RanimScene) {
    let song = Arc::new(parse_song());
    let layout = PianoLayout::new(&song);

    let mut notes = AnimStack::new();
    for &note in &song.notes {
        let note_eval = SingleNoteEval::new(note, layout);
        let start_sec = note_eval.start_sec();
        let duration_secs = note_eval.duration_secs();
        notes.push(
            note_eval
                .with_duration(duration_secs)
                .with_rate_func(linear)
                .at(start_sec),
        );
    }
    r.play(camera(song.duration_secs));
    r.play(Sound::new(render_song_audio(&song)));
    r.play(stack![notes, common_layers(&song, layout)]);
    r.insert_time_mark(
        song.duration_secs * 0.5,
        TimeMark::Capture("preview.png".to_string()),
    );
}

#[test]
fn bundled_midi_parses() {
    let song = parse_song();
    assert_eq!(song.notes.len(), 3378);
    assert!(song.duration_secs > 58.0);
}

#[test]
fn every_rendered_frame_has_a_camera() {
    // Regression: the synthesized clip is ceil-rounded up to whole samples,
    // which used to extend the scene duration a fraction of a frame past the
    // camera's window, leaving the last rendered frame camera-less (D0002).
    let song = Arc::new(parse_song());
    let layout = PianoLayout::new(&song);
    let mut r = RanimScene::new();
    r.play(camera(song.duration_secs));
    r.play(Sound::new(render_song_audio(&song)));
    r.play(stack![
        MidiNotesEval {
            song: song.clone(),
            layout
        }
        .with_duration(song.duration_secs)
        .with_rate_func(linear),
        common_layers(&song, layout),
    ]);
    let sealed = r.seal();
    let fps = 30.0;
    let raw = sealed.total_secs() * fps;
    let n = raw.ceil() as u64;
    let num_frames = if (raw - raw.round()).abs() < 1e-9 {
        n
    } else {
        n + 1
    };
    for f in 0..num_frames {
        let sec = (f as f64 / fps).min(sealed.total_secs());
        let cameras = sealed
            .eval_at_sec(sec)
            .filter(|(_, item)| matches!(item, ranim::core::core_item::CoreItem::CameraFrame(_)))
            .count();
        assert_eq!(cameras, 1, "frame {f} at {sec}s has {cameras} cameras");
    }
}
