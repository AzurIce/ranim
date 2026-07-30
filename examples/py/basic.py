"""Minimal ranimpy example, mirroring examples/getting_started0/lib.rs.

Run from the repository root:

    cargo run -p ranim-cli --features python -- render examples/py/basic.py
"""

import ranimpy as r

manim = r.palettes.manim
rf = r.rate_functions


@r.scene(output_dir="./output/basic_py")
def basic(scene: r.RanimScene):
    # A blue square writes itself, morphs into a red circle, then fades out.
    square = r.Square(2.0).set_color(manim.BLUE_C)
    circle = r.Circle(1.0).set_color(manim.RED_C)

    square_v = square.to_vitem()

    content = square_v.write().with_rate_func(rf.smooth)
    content.hold(0.5)
    content.push(square_v.morph_to(circle.to_vitem()).with_rate_func(rf.smooth))
    content.hold(0.5)
    morph_done_secs = content.duration
    content.push(square_v.fade_out().with_rate_func(rf.smooth))

    total_secs = content.duration
    scene.play(r.CameraFrame().show().with_duration(total_secs))
    scene.play(content)

    scene.insert_time_mark_capture(morph_done_secs, "preview.png")
