"""A composition-focused animation example, ported from
examples/composable_choreography/lib.rs.

Run from the repository root:

    cargo run -p ranim-cli --features python -- render examples/py/composable_choreography.py
"""

import ranimpy as r

manim = r.palettes.manim
rf = r.rate_functions

FADE_SECS = 0.35
MOVE_SECS = 0.7
HOLD_SECS = 0.25
WAVE_DELAY_SECS = 0.16

TILE_COLORS = [
    manim.BLUE_C,
    manim.TEAL_C,
    manim.GREEN_C,
    manim.YELLOW_C,
    manim.ORANGE,
    manim.RED_C,
]

Z_AXIS = (0.0, 0.0, 1.0)


def neg(v):
    return (-v[0], -v[1], -v[2])


def tile_row(y):
    tiles = []
    for i, color in enumerate(TILE_COLORS):
        square = (
            r.Square(0.85)
            .set_fill_color(color.with_alpha(0.72))
            .set_stroke_color(color)
            .set_stroke_width(0.055)
            .move_to((i * 1.35 - 3.375, y, 0.0))
        )
        tiles.append(square.to_vitem())
    return tiles


def tile_phrase(tile, shift, angle):
    """Build one reusable, self-contained phrase for a tile."""
    phrase = tile.fade_in().with_duration(FADE_SECS).with_rate_func(rf.smooth)

    # morph with a mutation closure -> clone, mutate, morph_to
    dst = tile.clone()
    dst.shift(shift).rotate_about(angle, Z_AXIS, dst.center)
    phrase.push(tile.morph_to(dst).with_duration(MOVE_SECS).with_rate_func(rf.smooth))
    phrase.hold(HOLD_SECS)

    dst.shift(neg(shift)).rotate_about(-angle, Z_AXIS, dst.center)
    phrase.push(tile.morph_to(dst).with_duration(MOVE_SECS).with_rate_func(rf.smooth))
    phrase.push(tile.fade_out().with_duration(FADE_SECS).with_rate_func(rf.smooth))
    return phrase


def wave(tiles, shift, angle):
    """A stack of staggered reusable phrases."""
    return r.stack(
        [tile_phrase(tile, shift, angle).at(i * WAVE_DELAY_SECS) for i, tile in enumerate(tiles)]
    )


@r.scene(output_dir="./output/composable_choreography_py", clear_color="#11131d")
def composable_choreography(scene: r.RanimScene):
    # Sequence -> Stack -> Sequence: a single staggered row.
    opening = wave(tile_row(0.0), (0.0, 0.9, 0.0), 0.35)

    # Sequence -> Stack -> Stack -> Sequence: two independently staggered waves.
    duet = r.stack(
        [
            wave(tile_row(-1.45), (0.85, 0.0, 0.0), -0.3),
            wave(tile_row(1.45), (-0.85, 0.0, 0.0), 0.3).at(0.45),
        ]
    )

    # Reuse the same `wave` factory for a denser finale.
    finale = r.stack(
        [
            wave(tile_row(0.0), (0.0, 1.15, 0.0), 0.48),
            wave(tile_row(0.0), (0.0, -1.15, 0.0), -0.48).at(0.32),
        ]
    )

    show = r.sequence([opening, duet])
    show.hold(0.4)
    show.push(finale)

    total_secs = show.duration
    scene.play(r.CameraFrame().show().with_duration(total_secs))
    scene.play(show)
    scene.insert_time_mark_capture(total_secs * 0.5, "preview.png")
