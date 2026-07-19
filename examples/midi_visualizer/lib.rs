//! Scrolling piano-roll visualization of the bundled Nyan Cat MIDI file.
//!
//! MIDI source: <https://freemidi.org/getter-25332>
//! See `SOURCE.md` for the download filename, retrieval date, and checksum.

mod midi;
mod visual;

use std::sync::Arc;

use midi::parse_song;
use ranim::{glam::DVec3, prelude::*, utils::rate_functions::linear};
use ranim_core::animation::{Eval, Placeable, StaticAnim};
use visual::{
    FRAME_HEIGHT, HitEffectsEval, MidiNotesEval, PianoKeyboardEval, PianoLayout, SingleNoteEval,
    make_background,
};

fn add_common_layers(r: &mut RanimScene, song: &Arc<midi::MidiSong>, layout: PianoLayout) {
    let duration = song.duration_secs;
    let background = make_background(layout);
    let camera = CameraFrame {
        pos: DVec3::ZERO,
        frame_height: FRAME_HEIGHT,
        ..Default::default()
    };

    r.play(background.show().with_duration(duration));
    r.play(
        PianoKeyboardEval {
            song: song.clone(),
            layout,
        }
        .into_animation_cell()
        .with_duration(duration)
        .with_rate_func(linear),
    );
    r.play(
        HitEffectsEval {
            song: song.clone(),
            layout,
        }
        .into_animation_cell()
        .with_duration(duration)
        .with_rate_func(linear),
    );
    r.play(camera.show().with_duration(duration));
    r.insert_time_mark(duration * 0.5, TimeMark::Capture("preview.png".to_string()));
}

#[scene(clear_color = "#070711")]
#[output(fps = 30, dir = "./output/midi_visualizer")]
fn midi_visualizer(r: &mut RanimScene) {
    let song = Arc::new(parse_song());
    let layout = PianoLayout::new(&song);
    let duration = song.duration_secs;

    r.play(
        MidiNotesEval {
            song: song.clone(),
            layout,
        }
        .into_animation_cell()
        .with_duration(duration)
        .with_rate_func(linear),
    );
    add_common_layers(r, &song, layout);
}

#[scene(clear_color = "#070711")]
#[output(fps = 30, dir = "./output/midi_visualizer")]
fn midi_visualizer_per_note(r: &mut RanimScene) {
    let song = Arc::new(parse_song());
    let layout = PianoLayout::new(&song);

    for &note in &song.notes {
        let note_eval = SingleNoteEval::new(note, layout);
        let start_sec = note_eval.start_sec();
        let duration_secs = note_eval.duration_secs();
        r.play(
            note_eval
                .into_animation_cell()
                .with_duration(duration_secs)
                .with_rate_func(linear)
                .at(start_sec),
        );
    }
    add_common_layers(r, &song, layout);
}

#[test]
fn bundled_midi_parses() {
    let song = parse_song();
    assert_eq!(song.notes.len(), 3378);
    assert!(song.duration_secs > 58.0);
}
