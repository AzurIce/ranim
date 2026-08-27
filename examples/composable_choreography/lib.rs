//! A composition-focused animation example.
//!
//! The scene deliberately uses reusable animation factories and nested
//! `AnimSequence` / `AnimStack` containers so its structure is visible in the
//! preview Timeline.

use ranim::{
    anims::{fading::FadingAnim, morph::MorphAnim},
    color::palettes::manim,
    glam::{DVec3, dvec3},
    items::vitem::{VItem, geometry::Square},
    prelude::*,
    utils::rate_functions::smooth,
};

const FADE_SECS: f64 = 0.35;
const MOVE_SECS: f64 = 0.7;
const HOLD_SECS: f64 = 0.25;
const WAVE_DELAY_SECS: f64 = 0.16;

/// Build one reusable, self-contained phrase for a tile.
///
/// Reuse happens by constructing a fresh `AnimSequence`, rather than cloning
/// an already-built animation tree.
fn tile_phrase(mut tile: VItem, shift: DVec3, angle: f64) -> AnimSequence {
    let mut phrase = seq![
        tile.fade_in()
            .with_duration(FADE_SECS)
            .with_rate_func(smooth),
    ];
    phrase
        .push(
            tile.morph(|item| {
                item.shift(shift);
                item.with_origin(AabbPoint::CENTER, |transform| {
                    transform.rotate_on_z(angle);
                });
            })
            .with_duration(MOVE_SECS)
            .with_rate_func(smooth),
        )
        .hold(HOLD_SECS)
        .push(
            tile.morph(|item| {
                item.with_origin(AabbPoint::CENTER, |transform| {
                    transform.rotate_on_z(-angle);
                });
                item.shift(-shift);
            })
            .with_duration(MOVE_SECS)
            .with_rate_func(smooth),
        )
        .push(
            tile.fade_out()
                .with_duration(FADE_SECS)
                .with_rate_func(smooth),
        );
    phrase
}

/// Construct a stack of staggered reusable phrases.
fn wave(tiles: impl IntoIterator<Item = VItem>, shift: DVec3, angle: f64) -> AnimStack {
    let mut result = AnimStack::new();

    for (index, tile) in tiles.into_iter().enumerate() {
        result.push(tile_phrase(tile, shift, angle).at(index as f64 * WAVE_DELAY_SECS));
    }

    result
}

fn tile_row(y: f64) -> Vec<VItem> {
    let colors = [
        manim::BLUE_C,
        manim::TEAL_C,
        manim::GREEN_C,
        manim::YELLOW_C,
        manim::ORANGE,
        manim::RED_C,
    ];

    colors
        .into_iter()
        .enumerate()
        .map(|(index, color)| {
            let mut square = VItem::from(Square::new(0.85).with(|square| {
                square
                    .set_fill_color(color.with_alpha(0.72))
                    .set_stroke_color(color);
                square.stroke_width = 0.055;
            }));
            square.shift(dvec3(index as f64 * 1.35 - 3.375, y, 0.0));
            square
        })
        .collect()
}

#[scene(clear_color = "#11131d")]
#[wasm_demo_doc]
#[output(dir = "./output/composable_choreography")]
fn composable_choreography(r: &mut RanimScene) {
    // Sequence -> Stack -> Sequence: a single staggered row.
    let opening = wave(tile_row(0.0), DVec3::Y * 0.9, 0.35);

    // Sequence -> Stack -> Stack -> Sequence: two independently staggered waves.
    let duet = stack![
        wave(tile_row(-1.45), DVec3::X * 0.85, -0.3),
        wave(tile_row(1.45), DVec3::NEG_X * 0.85, 0.3).at(0.45),
    ];

    // Reuse the same `wave` factory for a denser finale.
    let finale = stack![
        wave(tile_row(0.0), DVec3::Y * 1.15, 0.48),
        wave(tile_row(0.0), DVec3::NEG_Y * 1.15, -0.48).at(0.32),
    ];

    let mut show = seq![opening, duet];
    show.hold(0.4).push(finale);

    let duration_secs = show.duration_secs();
    r.play(CameraFrame::default().show().with_duration(duration_secs));
    r.play(show);
    r.insert_time_mark(
        duration_secs * 0.5,
        TimeMark::Capture("preview.png".to_owned()),
    );
}
