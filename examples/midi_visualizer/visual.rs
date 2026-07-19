use std::sync::Arc;

use ranim::{
    color::{AlphaColor, Srgb, palettes::manim},
    glam::{DVec3, dvec3},
    items::vitem::{VItem, geometry::Rectangle},
    prelude::*,
};
use ranim_core::animation::Eval;

use crate::midi::{MidiSong, Note};

const NOTE_SPEED: f64 = 5.5;
pub(crate) const FRAME_HEIGHT: f64 = 8.0;
const FRAME_WIDTH: f64 = FRAME_HEIGHT * 16.0 / 9.0;
const FRAME_LEFT: f64 = -FRAME_WIDTH * 0.5;
const FRAME_RIGHT: f64 = FRAME_WIDTH * 0.5;
const KEYBOARD_WIDTH: f64 = 1.45;
const HIT_X: f64 = FRAME_LEFT + KEYBOARD_WIDTH;
const KEY_ATTACK_SECS: f64 = 0.055;
const KEY_RELEASE_SECS: f64 = 0.12;
const HIT_EFFECT_SECS: f64 = 0.22;

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
pub(crate) struct PianoLayout {
    min_key: u8,
    max_key: u8,
    pitch_step: f64,
    note_height: f64,
    center_key: f64,
}

impl PianoLayout {
    pub(crate) fn new(song: &MidiSong) -> Self {
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

pub(crate) fn make_background(layout: PianoLayout) -> Vec<VItem> {
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

fn render_note(note: Note, sec: f64, layout: PianoLayout, items: &mut Vec<VItem>) {
    if note.end_sec <= sec {
        return;
    }

    let raw_left = HIT_X + (note.start_sec - sec) * NOTE_SPEED;
    let raw_right = HIT_X + (note.end_sec - sec) * NOTE_SPEED;
    if raw_right <= HIT_X || raw_left >= FRAME_RIGHT {
        return;
    }

    let left = raw_left.max(HIT_X);
    let right = raw_right.min(FRAME_RIGHT + 0.1);
    let width = right - left;
    if width <= 0.002 {
        return;
    }

    let hit_age = sec - note.start_sec;
    let hit_pulse = if (0.0..HIT_EFFECT_SECS).contains(&hit_age) {
        1.0 - smoothstep(hit_age / HIT_EFFECT_SECS)
    } else {
        0.0
    };
    let velocity = f64::from(note.velocity) / 127.0;
    let height = layout.note_height * (1.0 + 0.28 * hit_pulse * velocity);
    let opacity = (0.48 + 0.46 * velocity + 0.06 * hit_pulse) as f32;
    let y = layout.y(note.key);
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

pub(crate) struct MidiNotesEval {
    pub(crate) song: Arc<MidiSong>,
    pub(crate) layout: PianoLayout,
}

impl Eval<Vec<VItem>> for MidiNotesEval {
    fn eval_alpha(&self, alpha: f64) -> Vec<VItem> {
        let sec = alpha.clamp(0.0, 1.0) * self.song.duration_secs;
        let visible_until = sec + (FRAME_RIGHT - HIT_X) / NOTE_SPEED;
        let first_index = self
            .song
            .notes
            .partition_point(|note| note.start_sec < sec - self.song.max_note_duration_secs);
        let end_index = self
            .song
            .notes
            .partition_point(|note| note.start_sec <= visible_until);
        let mut items = Vec::new();

        for &note in &self.song.notes[first_index..end_index] {
            render_note(note, sec, self.layout, &mut items);
        }
        items
    }
}

pub(crate) struct SingleNoteEval {
    note: Note,
    layout: PianoLayout,
    start_sec: f64,
    duration_secs: f64,
}

impl SingleNoteEval {
    pub(crate) fn new(note: Note, layout: PianoLayout) -> Self {
        let visible_secs = (FRAME_RIGHT - HIT_X) / NOTE_SPEED;
        let start_sec = (note.start_sec - visible_secs).max(0.0);
        Self {
            note,
            layout,
            start_sec,
            duration_secs: note.end_sec - start_sec,
        }
    }

    pub(crate) fn start_sec(&self) -> f64 {
        self.start_sec
    }

    pub(crate) fn duration_secs(&self) -> f64 {
        self.duration_secs
    }
}

impl Eval<Vec<VItem>> for SingleNoteEval {
    fn eval_alpha(&self, alpha: f64) -> Vec<VItem> {
        let sec = self.start_sec + alpha.clamp(0.0, 1.0) * self.duration_secs;
        let mut items = Vec::with_capacity(2);
        render_note(self.note, sec, self.layout, &mut items);
        items
    }
}

pub(crate) struct PianoKeyboardEval {
    pub(crate) song: Arc<MidiSong>,
    pub(crate) layout: PianoLayout,
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

pub(crate) struct HitEffectsEval {
    pub(crate) song: Arc<MidiSong>,
    pub(crate) layout: PianoLayout,
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
