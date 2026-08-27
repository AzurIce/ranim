//! Z-fighting / draw-order verification.
//!
//! Ideal behavior: items occupying the same position are drawn in scene
//! insertion order (the order they were added to the scene), and the result
//! is stable (no per-pixel flicker between the two colors).
//!
//! Every item is played as its own cell (`r.play(item.show())`), so the frame
//! contains all items with scene order = play order. Pairs of coplanar items
//! sit at the same position; the *later-inserted* (blue) item should win
//! everywhere in the overlap, deterministically.
//!
//! Scenes:
//! - `opaque_front`       : straight-on camera (control — insertion order works).
//! - `opaque_angled`      : angled ortho camera — reproduction: the
//!   later-inserted blue circle loses chunks of the overlap to the red square.
//! - `translucent_front`  : translucent fills go through the OIT path, whose
//!   compositing order for depth ties is arrival order, not insertion order.
//! - `z_precision`        : z-gap sweep, straight-on (control — all clean).
//! - `z_precision_angled` : z-gap sweep, angled. Each square has its own
//!   color so the streaks identify which square's fragments win: the fighting
//!   appears for gaps around 1e-4..1e-2 and disappears again at 1e-1.
//! - `z_sweep`            : one pair whose z separation sweeps through zero.
//! - `flicker`            : camera orbiting one pair — the winning color
//!   changes over time on identical scene state (temporal instability).

use ranim::{
    anims::camera::CameraFrameAnim,
    color::{AlphaColor, Srgb, palettes::manim},
    glam::{DVec3, dvec3},
    items::vitem::geometry::{Circle, Square},
    prelude::*,
    utils::rate_functions::linear,
};

fn square(color: AlphaColor<Srgb>) -> Transformed<Square, Translation> {
    Square::new(2.0)
        .with(|s| {
            s.set_fill_color(color);
        })
        .transformed(Translation(dvec3(0.0, 0.0, 0.0)))
}

fn circle(color: AlphaColor<Srgb>) -> Transformed<Circle, Translation> {
    Circle::new(1.2)
        .with(|c| {
            c.set_fill_color(color);
        })
        .transformed(Translation(dvec3(0.0, 0.0, 0.0)))
}

/// Play `red` then `blue` at the same position `pos` for the whole scene.
///
/// Two separate cells: red has the lower scene order and should lose the
/// overlap against blue everywhere.
macro_rules! pair {
    ($r:expr, $red:expr, $blue:expr, $pos:expr, $total:expr) => {{
        let mut red = $red;
        let mut blue = $blue;
        red.transform = Translation($pos);
        blue.transform = Translation($pos);
        $r.play(red.show().with_duration($total));
        $r.play(blue.show().with_duration($total));
    }};
}

/// red square + blue circle pair, both lifted to `z`.
macro_rules! pair_z {
    ($r:expr, $x:expr, $z:expr, $total:expr) => {{
        pair!(
            $r,
            square(manim::RED_C),
            circle(manim::BLUE_C),
            dvec3($x, 0.0, $z),
            $total
        );
    }};
}

fn angled_ortho_camera() -> CameraFrame {
    let mut cam = CameraFrame::from_spherical(60f64.to_radians(), 30f64.to_radians(), 30.0);
    cam.perspective_blend = 0.0;
    cam
}

fn setup_camera(r: &mut RanimScene, dur: f64) {
    r.play(CameraFrame::default().show().with_duration(dur));
}

fn capture(r: &mut RanimScene, total: f64, name: &str) {
    r.insert_time_mark(total - 0.2, TimeMark::Capture(name.to_string()));
}

/// Straight-on ortho camera:
/// - left pair: identical squares at z=0 (control, insertion order)
/// - middle pair: square + circle at z=0
/// - right pair: square + circle at z=0.5
#[scene]
#[output(dir = "./output/z_fighting")]
fn opaque_front(r: &mut RanimScene) {
    let total = 2.2;
    setup_camera(r, total);

    pair!(
        r,
        square(manim::RED_C),
        square(manim::BLUE_C),
        dvec3(-4.5, 0.0, 0.0),
        total
    );
    pair_z!(r, 0.0, 0.0, total);
    pair_z!(r, 4.5, 0.5, total);

    capture(r, total, "opaque_front.png");
}

/// Same layout as `opaque_front`, but the ortho camera looks from an angle,
/// so the items' planes are no longer perpendicular to the view direction.
#[scene]
#[output(dir = "./output/z_fighting")]
fn opaque_angled(r: &mut RanimScene) {
    let total = 2.2;
    r.play(angled_ortho_camera().show().with_duration(total));

    pair!(
        r,
        square(manim::RED_C),
        square(manim::BLUE_C),
        dvec3(-4.5, 0.0, 0.0),
        total
    );
    pair_z!(r, 0.0, 0.0, total);
    pair_z!(r, 4.5, 0.5, total);

    capture(r, total, "opaque_angled.png");
}

/// Straight-on camera, translucent fills (OIT path):
/// - left pair: red then blue, both 50% alpha, same position
/// - right pair: opaque red, then 50% blue on top
#[scene]
#[output(dir = "./output/z_fighting")]
fn translucent_front(r: &mut RanimScene) {
    let total = 2.0;
    setup_camera(r, total);

    pair!(
        r,
        square(manim::RED_C.with_alpha(0.5)),
        square(manim::BLUE_C.with_alpha(0.5)),
        dvec3(-4.5, 0.0, 0.5),
        total
    );
    pair!(
        r,
        square(manim::RED_C),
        square(manim::BLUE_C.with_alpha(0.5)),
        dvec3(4.5, 0.0, 0.5),
        total
    );

    capture(r, total, "translucent_front.png");
}

/// square + circle pairs (different geometry, same plane) lifted to z=0.5,
/// with the blue circle separated by a growing z-gap, straight-on camera.
/// Control: insertion order holds for every gap.
#[scene]
#[output(dir = "./output/z_fighting")]
fn z_precision(r: &mut RanimScene) {
    let total = 3.4;
    setup_camera(r, total);

    let gaps = [0.0, 1e-5, 1e-4, 1e-3, 1e-2, 1e-1];
    for (i, gap) in gaps.iter().enumerate() {
        let x = -5.0 + 2.0 * i as f64;
        pair_z!(r, x, 0.5 + gap, total);
    }

    capture(r, total, "z_precision.png");
}

/// Same gaps as `z_precision` but seen from an angle. Each square has its own
/// fill color so the streaks identify which square's fragments win.
#[scene]
#[output(dir = "./output/z_fighting")]
fn z_precision_angled(r: &mut RanimScene) {
    let total = 3.4;
    r.play(angled_ortho_camera().show().with_duration(total));

    let gaps = [0.0, 1e-5, 1e-4, 1e-3, 1e-2, 1e-1];
    let sq_colors = [
        manim::RED_C,
        manim::GREEN_C,
        manim::YELLOW_C,
        manim::PURPLE_C,
        manim::MAROON_C,
        manim::GOLD_A,
    ];
    for (i, gap) in gaps.iter().enumerate() {
        let x = -5.0 + 2.0 * i as f64;
        pair!(
            r,
            square(sq_colors[i]),
            circle(manim::BLUE_C),
            dvec3(x, 0.0, 0.5 + gap),
            total
        );
    }

    capture(r, total, "z_precision_angled.png");
}

/// Fixed angled camera; the blue circle's z sweeps linearly through the red
/// square's plane (z 0.45 -> 0.55, square at 0.5). The winner flips cleanly
/// at delta = 0 at this screen position; the same gap fights at others.
#[scene]
#[output(dir = "./output/z_fighting")]
fn z_sweep(r: &mut RanimScene) {
    let total = 4.0;
    r.play(angled_ortho_camera().show().with_duration(total));

    let mut sq = square(manim::RED_C);
    sq.transform = Translation(dvec3(0.0, 0.0, 0.5));
    r.play(sq.show().with_duration(total));

    r.play(
        Pure::new(|alpha| {
            let z = 0.45 + 0.1 * alpha;
            let mut ci = circle(manim::BLUE_C);
            ci.transform = Translation(dvec3(0.0, 0.0, z));
            ci
        })
        .with_duration(total)
        .with_rate_func(linear),
    );
}

/// Slowly orbit a perspective camera around the square+circle pair to show
/// that the winning color changes over time (instability).
#[scene]
#[output(dir = "./output/z_fighting")]
fn flicker(r: &mut RanimScene) {
    let total = 4.0;
    let mut cam = CameraFrame::from_spherical(60f64.to_radians(), 20f64.to_radians(), 8.0);
    cam.fovy = 50f64.to_radians();
    r.play(
        cam.orbit(DVec3::ZERO, 0.4)
            .with_duration(total)
            .with_rate_func(linear),
    );
    pair_z!(r, 0.0, 0.5, total);
}
