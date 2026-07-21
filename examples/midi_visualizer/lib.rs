//! Scrolling piano-roll visualization of the bundled Nyan Cat MIDI file.
//!
//! MIDI source: <https://freemidi.org/getter-25332>
//! See `SOURCE.md` for the download filename, retrieval date, and checksum.

mod midi;
mod visual;

use std::sync::Arc;

use midi::parse_song;
use ranim::{glam::DVec3, prelude::*, utils::rate_functions::linear};
use ranim_core::animation::{Placeable, StaticAnim};
use visual::{
    FRAME_HEIGHT, HitEffectsEval, MidiNotesEval, PianoKeyboardEval, PianoLayout, SingleNoteEval,
    make_background,
};

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

fn camera(duration: f64) -> impl Animation {
    CameraFrame {
        pos: DVec3::ZERO,
        frame_height: FRAME_HEIGHT,
        ..Default::default()
    }
    .show()
    .with_duration(duration)
}

#[scene(clear_color = "#070711")]
#[output(fps = 30, dir = "./output/midi_visualizer")]
fn midi_visualizer(r: &mut RanimScene) {
    let song = Arc::new(parse_song());
    let layout = PianoLayout::new(&song);
    let duration = song.duration_secs;

    r.play(camera(duration));
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
