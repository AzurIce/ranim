//! Audio demo: sounds declared in the same composition tree as animations.
//!
//! What to notice:
//! - `pop` / `melody` / `ticks` are declared like any animation and placed
//!   with `.at(...)`, reusing time coordinates taken *from the visual
//!   sub-trees* (`intro.cursor_sec()`), so sound and picture stay aligned by
//!   construction;
//! - the tick sounds reuse the exact same `lagged!` structure and ratio as
//!   the staggered creations, so each tick fires exactly when its shape
//!   appears;
//! - the output mp4 carries a 48 kHz stereo AAC track mixed from the sealed
//!   scene's audio plan.
//!
//! Render with: `cargo run -p ranim-cli -- output audio_demo --example audio_demo`

use std::{f32::consts::PI, f64::consts::TAU, path::PathBuf};

use ranim::{
    WavSource,
    anims::{creation::Create, fading::FadingAnim, morph::MorphAnim},
    color::palettes::manim,
    items::vitem::{
        VItem,
        geometry::{Circle, Square},
    },
    prelude::*,
};

/// A short synthesized note: a decay envelope times a sine at `freq`.
///
/// Procedural content is a pure function of the leaf's progress; the real
/// window length is attached with `with_duration`.
fn tone(freq: f32, secs: f64) -> impl Placeable {
    sound(Synth::new(move |alpha| {
        let env = (1.0 - alpha) as f32;
        let v = (alpha as f32 * freq * 2.0 * PI).sin() * env * env * 0.35;
        StereoFrame::splat(v)
    }))
    .with_duration(secs)
}

/// A high "tick" for the staggered creations.
fn tick() -> impl Placeable {
    tone(1568.0, 0.12)
}

/// Generate the pop sound effect once, so the demo needs no binary assets.
fn ensure_pop_asset() -> PathBuf {
    let dir = PathBuf::from("./output/audio_demo");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("pop.wav");
    if !path.exists() {
        let rate = 48000.0f64;
        let n = (rate * 0.25) as usize;
        let mut samples = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f64 / rate;
            let progress = i as f64 / n as f64;
            let freq = 220.0 + 880.0 * (1.0 - progress);
            let env = (1.0 - progress).powi(2);
            let v = (TAU * freq * t).sin() * env * 0.8;
            samples.push((v * 32767.0) as i16);
        }
        write_wav16(&path, &samples, rate as u32);
    }
    path
}

/// Minimal 16-bit mono wav writer.
fn write_wav16(path: &PathBuf, samples: &[i16], rate: u32) {
    use std::io::Write as _;
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + samples.len() as u32 * 2).to_le_bytes())
        .unwrap();
    f.write_all(b"WAVEfmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
    f.write_all(&1u16.to_le_bytes()).unwrap(); // mono
    f.write_all(&rate.to_le_bytes()).unwrap();
    f.write_all(&(rate * 2).to_le_bytes()).unwrap();
    f.write_all(&2u16.to_le_bytes()).unwrap();
    f.write_all(&16u16.to_le_bytes()).unwrap();
    f.write_all(b"data").unwrap();
    f.write_all(&(samples.len() as u32 * 2).to_le_bytes())
        .unwrap();
    for s in samples {
        f.write_all(&s.to_le_bytes()).unwrap();
    }
}

#[scene]
#[output(dir = "./output/audio_demo")]
fn audio_demo(r: &mut RanimScene) {
    let square = Square::new(2.0).with(|s| {
        s.set_color(manim::BLUE_C);
    });
    let circle: VItem = Circle::new(2.0)
        .with(|c| {
            c.set_color(manim::RED_C);
        })
        .into();
    let mut vitem = VItem::from(square.clone());

    // Visuals: fade in, morph, then three staggered creations.
    let intro = seq![
        square.clone().fade_in().with_duration(1.0),
        vitem.morph_to(circle).with_duration(1.0),
    ];
    let intro_secs = intro.cursor_sec();
    let staggered = lagged![
        0.3;
        Create::new(VItem::from(square)).with_duration(0.6),
        Create::new(VItem::from(
            Circle::new(0.6).with(|c| { c.set_color(manim::YELLOW_C); })
        ))
        .with_duration(0.6),
        Create::new(VItem::from(
            Circle::new(1.2).with(|c| { c.set_color(manim::GREEN_C); })
        ))
        .with_duration(0.6),
    ];
    // The staggered section simply follows the intro in sequence time.
    let visuals = seq![intro, staggered];

    // Audio shares the same coordinates: the pop lands exactly when the
    // fade-in completes, the melody starts with the staggered section, and
    // the ticks reuse its `lagged!` structure so each one fires with its
    // shape.
    let pop = WavSource::from_path(ensure_pop_asset()).unwrap();
    let audio = stack![
        pop.at(1.0),
        seq![
            tone(523.25, 0.2),
            tone(659.25, 0.2),
            tone(783.99, 0.2),
            tone(1046.50, 0.2),
        ]
        .at(intro_secs),
        lagged![0.3; tick(), tick(), tick()].at(intro_secs),
    ];

    let content = stack![visuals, audio];
    r.play(
        CameraFrame::default()
            .show()
            .with_duration(content.duration_secs()),
    );
    r.play(content);
}
