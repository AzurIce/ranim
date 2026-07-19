//! Scrolling piano-roll visualization of the bundled Nyan Cat MIDI file.
//!
//! MIDI source: <https://freemidi.org/getter-25332>
//! See `SOURCE.md` for the download filename, retrieval date, and checksum.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use ranim::{
    color::{AlphaColor, Srgb, palettes::manim},
    glam::{DVec3, dvec3},
    items::vitem::{VItem, geometry::Rectangle},
    prelude::*,
    utils::rate_functions::linear,
};
use ranim_core::animation::{Eval, StaticAnim};

const MIDI_BYTES: &[u8] = include_bytes!("nyan-cat.mid");
const NOTE_SPEED: f64 = 5.5;
const FRAME_HEIGHT: f64 = 8.0;
const FRAME_WIDTH: f64 = FRAME_HEIGHT * 16.0 / 9.0;
const FRAME_LEFT: f64 = -FRAME_WIDTH * 0.5;
const FRAME_RIGHT: f64 = FRAME_WIDTH * 0.5;
const KEYBOARD_WIDTH: f64 = 1.45;
const HIT_X: f64 = FRAME_LEFT + KEYBOARD_WIDTH;
const KEY_ATTACK_SECS: f64 = 0.055;
const KEY_RELEASE_SECS: f64 = 0.12;
const HIT_EFFECT_SECS: f64 = 0.22;

#[derive(Debug, Clone, Copy)]
struct RawNote {
    start_tick: u64,
    end_tick: u64,
    key: u8,
    velocity: u8,
    track: usize,
}

#[derive(Debug, Clone, Copy)]
struct Note {
    start_sec: f64,
    end_sec: f64,
    key: u8,
    velocity: u8,
    track: usize,
}

struct MidiSong {
    notes: Vec<Note>,
    notes_by_key: Vec<Vec<Note>>,
    duration_secs: f64,
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

fn parse_song() -> MidiSong {
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

    MidiSong {
        notes,
        notes_by_key,
        duration_secs,
    }
}

fn rectangle(
    width: f64,
    height: f64,
    position: DVec3,
    color: AlphaColor<Srgb>,
    opacity: f32,
) -> VItem {
    Rectangle::new(width, height)
        .with(|rectangle| {
            rectangle
                .set_stroke_opacity(0.0)
                .set_fill_color(color)
                .set_fill_opacity(opacity)
                .move_to(position);
        })
        .into()
}

fn track_color(track: usize) -> AlphaColor<Srgb> {
    const COLORS: [AlphaColor<Srgb>; 11] = [
        manim::BLUE_C,
        manim::TEAL_C,
        manim::GREEN_C,
        manim::YELLOW_C,
        manim::GOLD_C,
        manim::ORANGE,
        manim::RED_C,
        manim::MAROON_C,
        manim::PURPLE_C,
        manim::PINK,
        manim::LIGHT_PINK,
    ];
    COLORS[track % COLORS.len()]
}

fn smoothstep(alpha: f64) -> f64 {
    let alpha = alpha.clamp(0.0, 1.0);
    alpha * alpha * (3.0 - 2.0 * alpha)
}

#[derive(Clone, Copy)]
struct PianoLayout {
    min_key: u8,
    max_key: u8,
    pitch_step: f64,
    note_height: f64,
    center_key: f64,
}

impl PianoLayout {
    fn new(song: &MidiSong) -> Self {
        let min_key = song.notes.iter().map(|note| note.key).min().unwrap_or(48);
        let max_key = song.notes.iter().map(|note| note.key).max().unwrap_or(84);
        let pitch_count = u16::from(max_key) - u16::from(min_key) + 1;
        let pitch_step = (FRAME_HEIGHT - 0.5) / f64::from(pitch_count);
        Self {
            min_key,
            max_key,
            pitch_step,
            note_height: pitch_step * 0.76,
            center_key: (f64::from(min_key) + f64::from(max_key)) * 0.5,
        }
    }

    fn y(self, key: u8) -> f64 {
        (f64::from(key) - self.center_key) * self.pitch_step
    }
}

fn is_black_key(key: u8) -> bool {
    matches!(key % 12, 1 | 3 | 6 | 8 | 10)
}

fn make_background(layout: PianoLayout) -> Vec<VItem> {
    let roll_width = FRAME_RIGHT - HIT_X;
    let roll_center = (HIT_X + FRAME_RIGHT) * 0.5;
    let mut background = vec![rectangle(
        roll_width,
        FRAME_HEIGHT,
        dvec3(roll_center, 0.0, -0.05),
        manim::GREY_E,
        0.08,
    )];

    for key in layout.min_key..=layout.max_key {
        let y = layout.y(key);
        if is_black_key(key) {
            background.push(rectangle(
                roll_width,
                layout.pitch_step,
                dvec3(roll_center, y, -0.04),
                manim::GREY_D,
                0.14,
            ));
        }
        if key % 12 == 0 {
            background.push(rectangle(
                roll_width,
                0.018,
                dvec3(roll_center, y - layout.pitch_step * 0.5, -0.03),
                manim::GREY_B,
                0.42,
            ));
        }
    }

    background.push(rectangle(
        0.045,
        FRAME_HEIGHT,
        dvec3(HIT_X, 0.0, 0.15),
        manim::WHITE,
        0.9,
    ));
    background
}

struct MidiNotesEval {
    song: Arc<MidiSong>,
    layout: PianoLayout,
}

impl Eval<Vec<VItem>> for MidiNotesEval {
    fn eval_alpha(&self, alpha: f64) -> Vec<VItem> {
        let sec = alpha.clamp(0.0, 1.0) * self.song.duration_secs;
        let visible_until = sec + (FRAME_RIGHT - HIT_X) / NOTE_SPEED;
        let end_index = self
            .song
            .notes
            .partition_point(|note| note.start_sec <= visible_until);
        let mut items = Vec::new();

        for note in &self.song.notes[..end_index] {
            if note.end_sec <= sec {
                continue;
            }

            let raw_left = HIT_X + (note.start_sec - sec) * NOTE_SPEED;
            let raw_right = HIT_X + (note.end_sec - sec) * NOTE_SPEED;
            if raw_right <= HIT_X || raw_left >= FRAME_RIGHT {
                continue;
            }

            let left = raw_left.max(HIT_X);
            let right = raw_right.min(FRAME_RIGHT + 0.1);
            let width = right - left;
            if width <= 0.002 {
                continue;
            }

            let hit_age = sec - note.start_sec;
            let hit_pulse = if (0.0..HIT_EFFECT_SECS).contains(&hit_age) {
                1.0 - smoothstep(hit_age / HIT_EFFECT_SECS)
            } else {
                0.0
            };
            let velocity = f64::from(note.velocity) / 127.0;
            let height = self.layout.note_height * (1.0 + 0.28 * hit_pulse * velocity);
            let opacity = (0.48 + 0.46 * velocity + 0.06 * hit_pulse) as f32;
            let y = self.layout.y(note.key);
            let z = 0.2 + note.track as f64 * 0.0002;

            items.push(rectangle(
                width,
                height,
                dvec3((left + right) * 0.5, y, z),
                track_color(note.track),
                opacity,
            ));

            if hit_pulse > 0.0 {
                items.push(rectangle(
                    0.035 + 0.08 * hit_pulse,
                    height * (1.0 + 0.5 * hit_pulse),
                    dvec3(HIT_X + 0.025, y, z + 0.02),
                    manim::WHITE,
                    (0.2 + 0.7 * hit_pulse * velocity) as f32,
                ));
            }
        }

        items
    }
}

struct PianoKeyboardEval {
    song: Arc<MidiSong>,
    layout: PianoLayout,
}

impl PianoKeyboardEval {
    fn key_state(&self, key: u8, sec: f64) -> (f64, usize) {
        let mut strongest = 0.0_f64;
        let mut strongest_track = 0;

        for note in &self.song.notes_by_key[usize::from(key)] {
            if note.start_sec > sec {
                break;
            }
            if sec >= note.end_sec + KEY_RELEASE_SECS {
                continue;
            }

            let envelope = if sec < note.start_sec + KEY_ATTACK_SECS {
                smoothstep((sec - note.start_sec) / KEY_ATTACK_SECS)
            } else if sec <= note.end_sec {
                1.0
            } else {
                1.0 - smoothstep((sec - note.end_sec) / KEY_RELEASE_SECS)
            };
            let pressure = envelope * (0.5 + 0.5 * f64::from(note.velocity) / 127.0);
            if pressure > strongest {
                strongest = pressure;
                strongest_track = note.track;
            }
        }

        (strongest, strongest_track)
    }
}

impl Eval<Vec<VItem>> for PianoKeyboardEval {
    fn eval_alpha(&self, alpha: f64) -> Vec<VItem> {
        let sec = alpha.clamp(0.0, 1.0) * self.song.duration_secs;
        let mut keys = Vec::new();

        for key in self.layout.min_key..=self.layout.max_key {
            let black = is_black_key(key);
            let (pressure, track) = self.key_state(key, sec);
            let width = if black {
                KEYBOARD_WIDTH * 0.72
            } else {
                KEYBOARD_WIDTH
            };
            let pressed_offset = 0.095 * pressure;
            let x = HIT_X - width * 0.5 - pressed_offset;
            let y = self.layout.y(key);
            let height = self.layout.pitch_step * if black { 0.78 } else { 0.92 };
            let z = if black { 0.34 } else { 0.3 };
            let base_color = if black { manim::GREY_E } else { manim::GREY_B };

            keys.push(rectangle(width, height, dvec3(x, y, z), base_color, 0.96));

            if pressure > 0.0 {
                keys.push(rectangle(
                    width,
                    height,
                    dvec3(x, y, z + 0.01),
                    track_color(track),
                    (0.18 + 0.72 * pressure) as f32,
                ));
                keys.push(rectangle(
                    0.065,
                    height * 0.86,
                    dvec3(HIT_X - pressed_offset - 0.04, y, z + 0.02),
                    manim::WHITE,
                    (0.2 + 0.65 * pressure) as f32,
                ));
            }
        }

        keys
    }
}

struct HitEffectsEval {
    song: Arc<MidiSong>,
    layout: PianoLayout,
}

impl Eval<Vec<VItem>> for HitEffectsEval {
    fn eval_alpha(&self, alpha: f64) -> Vec<VItem> {
        let sec = alpha.clamp(0.0, 1.0) * self.song.duration_secs;
        let first = self
            .song
            .notes
            .partition_point(|note| note.start_sec < sec - HIT_EFFECT_SECS);
        let last = self
            .song
            .notes
            .partition_point(|note| note.start_sec <= sec);
        let mut effects = Vec::new();

        for note in &self.song.notes[first..last] {
            let progress = ((sec - note.start_sec) / HIT_EFFECT_SECS).clamp(0.0, 1.0);
            let opacity = 1.0 - smoothstep(progress);
            let velocity = f64::from(note.velocity) / 127.0;
            effects.push(rectangle(
                0.06 + 0.34 * progress,
                self.layout.note_height * (1.2 + 3.5 * progress),
                dvec3(HIT_X + 0.04 * progress, self.layout.y(note.key), 0.42),
                track_color(note.track),
                (opacity * velocity * 0.62) as f32,
            ));
        }

        effects
    }
}

#[scene(clear_color = "#070711")]
#[output(fps = 30, dir = "./output/midi_visualizer")]
fn midi_visualizer(r: &mut RanimScene) {
    let song = Arc::new(parse_song());
    let layout = PianoLayout::new(&song);
    let duration = song.duration_secs;
    let background = make_background(layout);
    let camera = CameraFrame {
        pos: DVec3::ZERO,
        frame_height: FRAME_HEIGHT,
        ..Default::default()
    };

    r.play(background.show().with_duration(duration));
    r.play(
        MidiNotesEval {
            song: song.clone(),
            layout,
        }
        .into_animation_cell()
        .with_duration(duration)
        .with_rate_func(linear),
    );
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

#[test]
fn bundled_midi_parses() {
    let song = parse_song();
    assert!(!song.notes.is_empty());
    assert!(song.duration_secs > 0.0);
}
