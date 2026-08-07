use std::collections::{BTreeMap, HashMap};

use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

const MIDI_BYTES: &[u8] = include_bytes!("nyan-cat.mid");

#[derive(Debug, Clone, Copy)]
struct RawNote {
    start_tick: u64,
    end_tick: u64,
    key: u8,
    velocity: u8,
    track: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Note {
    pub(crate) start_sec: f64,
    pub(crate) end_sec: f64,
    pub(crate) key: u8,
    pub(crate) velocity: u8,
    pub(crate) track: usize,
}

pub(crate) struct MidiSong {
    pub(crate) notes: Vec<Note>,
    pub(crate) notes_by_key: Vec<Vec<Note>>,
    pub(crate) max_note_duration_secs: f64,
    pub(crate) duration_secs: f64,
}

fn close_note(
    active: &mut HashMap<(u8, u8), Vec<(u64, u8)>>,
    channel: u8,
    key: u8,
    end_tick: u64,
    track: usize,
    notes: &mut Vec<RawNote>,
) {
    let Some(starts) = active.get_mut(&(channel, key)) else {
        return;
    };
    let Some((start_tick, velocity)) = starts.pop() else {
        return;
    };
    notes.push(RawNote {
        start_tick,
        end_tick: end_tick.max(start_tick + 1),
        key,
        velocity,
        track,
    });
}

fn tick_to_seconds(tick: u64, tempos: &[(u64, u32)], ticks_per_beat: u64) -> f64 {
    let mut seconds = 0.0;
    let mut cursor_tick = 0;
    let mut micros_per_beat = 500_000_u32;

    for &(tempo_tick, next_tempo) in tempos {
        if tempo_tick > tick {
            break;
        }
        seconds += (tempo_tick - cursor_tick) as f64 * micros_per_beat as f64
            / ticks_per_beat as f64
            / 1_000_000.0;
        cursor_tick = tempo_tick;
        micros_per_beat = next_tempo;
    }

    seconds
        + (tick - cursor_tick) as f64 * micros_per_beat as f64 / ticks_per_beat as f64 / 1_000_000.0
}

pub(crate) fn parse_song() -> MidiSong {
    let smf = Smf::parse(MIDI_BYTES).expect("the bundled MIDI file should be valid");
    let ticks_per_beat = match smf.header.timing {
        Timing::Metrical(ticks) => u64::from(ticks.as_int()),
        Timing::Timecode(_, _) => panic!("timecode-based MIDI is not supported by this example"),
    };

    let mut tempo_map = BTreeMap::new();
    let mut raw_notes = Vec::new();
    let mut song_end_tick = 0;

    for (track_index, track) in smf.tracks.iter().enumerate() {
        let mut tick = 0_u64;
        let mut active = HashMap::<(u8, u8), Vec<(u64, u8)>>::new();

        for event in track {
            tick += u64::from(event.delta.as_int());
            song_end_tick = song_end_tick.max(tick);

            match event.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(tempo)) => {
                    tempo_map.insert(tick, tempo.as_int());
                }
                TrackEventKind::Midi { channel, message } => match message {
                    MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                        active
                            .entry((channel.as_int(), key.as_int()))
                            .or_default()
                            .push((tick, vel.as_int()));
                    }
                    MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
                        close_note(
                            &mut active,
                            channel.as_int(),
                            key.as_int(),
                            tick,
                            track_index,
                            &mut raw_notes,
                        );
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        for ((_, key), starts) in active {
            for (start_tick, velocity) in starts {
                raw_notes.push(RawNote {
                    start_tick,
                    end_tick: song_end_tick.max(start_tick + 1),
                    key,
                    velocity,
                    track: track_index,
                });
            }
        }
    }

    let tempos = tempo_map.into_iter().collect::<Vec<_>>();
    let duration_secs = tick_to_seconds(song_end_tick, &tempos, ticks_per_beat);
    let mut notes = raw_notes
        .into_iter()
        .map(|note| Note {
            start_sec: tick_to_seconds(note.start_tick, &tempos, ticks_per_beat),
            end_sec: tick_to_seconds(note.end_tick, &tempos, ticks_per_beat),
            key: note.key,
            velocity: note.velocity,
            track: note.track,
        })
        .collect::<Vec<_>>();
    notes.sort_by(|a, b| a.start_sec.total_cmp(&b.start_sec));

    let mut notes_by_key = vec![Vec::new(); 128];
    for &note in &notes {
        notes_by_key[usize::from(note.key)].push(note);
    }
    let max_note_duration_secs = notes
        .iter()
        .map(|note| note.end_sec - note.start_sec)
        .fold(0.0_f64, f64::max);

    MidiSong {
        notes,
        notes_by_key,
        max_note_duration_secs,
        duration_secs,
    }
}
