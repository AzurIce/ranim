//! A scene with a synthesized soundtrack: sounds compose in the same tree as
//! visual animations (`seq!`/`stack!`/`at`) and flatten into the audio plane
//! at seal time — muxed into the rendered video, or played as the preview
//! clock.

use ranim::{
    anims::{creation::WritingAnim, fading::FadingAnim},
    color::palettes::manim,
    items::vitem::{VItem, geometry::Square},
    prelude::*,
    utils::rate_functions::smooth,
};

/// A little arpeggio, synthesized from pure sines (no audio assets).
fn arpeggio(notes: &[f64], note_secs: f64, repeats: usize) -> AudioClip {
    let sample_rate = 48_000_u32;
    let note_len = (note_secs * sample_rate as f64) as usize;
    let mut pcm = Vec::with_capacity(notes.len() * note_len * repeats);
    for _ in 0..repeats {
        for &freq in notes {
            for j in 0..note_len {
                let t = j as f64 / sample_rate as f64;
                // Fade each note in and out so consecutive notes don't click.
                let env = (t / note_secs).min(1.0) * (1.0 - t / note_secs);
                let wave = (2.0 * std::f64::consts::PI * freq * t).sin();
                pcm.push((0.35 * env * wave) as f32);
            }
        }
    }
    AudioClip::from_pcm(pcm, sample_rate, 1)
}

#[scene]
#[wasm_demo_doc]
#[output(dir = "./output/audio_demo")]
fn audio_demo(r: &mut RanimScene) {
    let mut square = Square::new(2.0);
    square.set_color(manim::BLUE_C);
    let mut vitem = VItem::from(square.clone());

    // The soundtrack lives in the same tree as the visuals: a looping
    // arpeggio as BGM under the whole choreography, plus a "ding" placed
    // exactly where the square finishes fading in.
    let bgm = Sound::new(arpeggio(&[261.63, 329.63, 392.0, 523.25], 0.3, 4))
        .with_fade_in(0.5)
        .with_fade_out(1.0);
    let ding = Sound::new(arpeggio(&[1046.5], 0.4, 1))
        .with_gain(0.8)
        .at(1.0);

    let visual = stack![
        seq![
            square.fade_in().with_rate_func(smooth),
            vitem.show().with_duration(1.0),
            vitem.clone().unwrite().with_rate_func(smooth),
            vitem.write().with_rate_func(smooth),
            vitem.fade_out().with_rate_func(smooth),
        ],
        ding,
    ];

    r.play(
        CameraFrame::default()
            .show()
            .with_duration(visual.duration_secs()),
    );
    r.play(visual);
    r.play(bgm);

    r.insert_time_mark(1.0, TimeMark::Capture("preview.png".to_string()));
}
