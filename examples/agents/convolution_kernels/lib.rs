//! Convolution kernels — how common 3x3 convolution kernels transform an
//! image, visualized on a pixel grid.
//!
//! A procedural 12x12 grayscale image (a smooth gradient, a bright disk and
//! a diagonal bar, so both sharp edges and smooth regions are present) is
//! shown as a grid of gray squares on the left. For each of four common
//! kernels — identity, box blur, sharpen and Laplacian edge detection — the
//! kernel matrix appears in the middle, a yellow 3x3 window slides over the
//! input in raster order, and each output pixel fades in on the right as it
//! is computed (zero padding, results clamped to [0, 1]; the edge kernel
//! shows |v|). A final summary row compares the input and all four outputs.
//!
//! Every item owns an [`AnimSequence`] on a shared timeline (the same
//! pattern as `examples/agents/rubiks_cube`), aligned with `forward_to` /
//! `hold_to`. The sliding window is a custom [`Eval`] (`ScanWindow`) that
//! snaps the highlight square to the pixel grid, like `CubieTurn` in the
//! Rubik's cube example.

use ranim::{
    anims::fading::{FadeOut, FadingAnim},
    color::{AlphaColor, Srgb, palettes::manim, rgb8},
    glam::{DVec3, dvec3},
    items::vitem::{VItem, geometry::Square, text::TextItem},
    prelude::*,
    utils::rate_functions::linear,
};

// MARK: Image and kernels

/// Image resolution: N x N pixels.
const N: usize = 12;

/// The source image: a horizontal gradient as smooth base, a bright disk
/// and a diagonal bar as sharp features.
fn image_value(row: usize, col: usize) -> f64 {
    let x = col as f64 / (N - 1) as f64;
    let y = row as f64 / (N - 1) as f64;
    let mut v = 0.15 + 0.35 * x;
    let (dx, dy) = (x - 0.40, y - 0.42);
    if dx * dx + dy * dy < 0.24 * 0.24 {
        v = 0.95;
    }
    if (x - y).abs() < 0.05 {
        v = v.max(0.75);
    }
    v.clamp(0.0, 1.0)
}

/// A common 3x3 convolution kernel with its display labels.
struct Kernel {
    name: &'static str,
    values: [[f64; 3]; 3],
    labels: [[&'static str; 3]; 3],
    /// Show |v| instead of clamping v (for edge detectors centered at 0).
    abs_output: bool,
}

const KERNELS: [Kernel; 4] = [
    Kernel {
        name: "Identity",
        values: [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]],
        labels: [["0", "0", "0"], ["0", "1", "0"], ["0", "0", "0"]],
        abs_output: false,
    },
    Kernel {
        name: "Box Blur",
        values: [[1.0 / 9.0; 3]; 3],
        labels: [
            ["1/9", "1/9", "1/9"],
            ["1/9", "1/9", "1/9"],
            ["1/9", "1/9", "1/9"],
        ],
        abs_output: false,
    },
    Kernel {
        name: "Sharpen",
        values: [[0.0, -1.0, 0.0], [-1.0, 5.0, -1.0], [0.0, -1.0, 0.0]],
        labels: [["0", "-1", "0"], ["-1", "5", "-1"], ["0", "-1", "0"]],
        abs_output: false,
    },
    Kernel {
        name: "Edge Detect",
        values: [[-1.0, -1.0, -1.0], [-1.0, 8.0, -1.0], [-1.0, -1.0, -1.0]],
        labels: [["-1", "-1", "-1"], ["-1", "8", "-1"], ["-1", "-1", "-1"]],
        abs_output: true,
    },
];

/// Convolve the source image with `kernel` (zero padding), returning the
/// N*N display values clamped to [0, 1].
fn convolve(kernel: &Kernel) -> Vec<f64> {
    let mut out = vec![0.0; N * N];
    for r in 0..N {
        for c in 0..N {
            let mut acc = 0.0;
            for kr in 0..3 {
                for kc in 0..3 {
                    let ir = r as i64 + kr as i64 - 1;
                    let ic = c as i64 + kc as i64 - 1;
                    if ir >= 0 && ir < N as i64 && ic >= 0 && ic < N as i64 {
                        acc += kernel.values[kr][kc] * image_value(ir as usize, ic as usize);
                    }
                }
            }
            let v = if kernel.abs_output { acc.abs() } else { acc };
            out[r * N + c] = v.clamp(0.0, 1.0);
        }
    }
    out
}

fn gray(v: f64) -> AlphaColor<Srgb> {
    let g = (v * 255.0).round() as u8;
    rgb8(g, g, g)
}

// MARK: Layout

/// Cell size of the input/output grids.
const CELL: f64 = 0.26;
const INPUT_CENTER: DVec3 = dvec3(-4.2, -0.2, 0.0);
const OUTPUT_CENTER: DVec3 = dvec3(4.2, -0.2, 0.0);
/// Cell size of the kernel matrix display.
const KCELL: f64 = 0.62;
const KERNEL_CENTER: DVec3 = dvec3(0.0, 0.25, 0.0);
const TITLE_POS: DVec3 = dvec3(0.0, 3.3, 0.0);

/// Center of cell (`row`, `col`) of an `n`x`n` grid of `cell`-sized squares
/// centered at `grid_center`.
fn cell_center(grid_center: DVec3, n: usize, cell: f64, row: usize, col: usize) -> DVec3 {
    let half = (n as f64 - 1.0) / 2.0;
    grid_center + dvec3((col as f64 - half) * cell, (half - row as f64) * cell, 0.0)
}

/// A single grayscale pixel square (no stroke; the gaps read as the grid).
fn pixel_square(center: DVec3, size: f64, v: f64) -> VItem {
    let sq = Square::new(size * 0.92).with(|sq| {
        sq.center = center;
    });
    let mut item = VItem::from(sq);
    item.set_fill_color(gray(v));
    item.set_stroke_opacity(0.0);
    item
}

/// A single-line text as glyphs, centered at `pos`.
fn text_vitems(text: &str, em_size: f64, pos: DVec3) -> Vec<VItem> {
    let mut vitems = Vec::<VItem>::from(TextItem::new(text, em_size));
    vitems.move_anchor_to(AabbPoint::CENTER, pos);
    vitems
}

// MARK: Timeline

const INTRO: f64 = 1.2;
/// Per kernel: matrix fades in, then the scan runs, the result holds and
/// everything fades out again.
const MATRIX_IN: f64 = 1.0;
const SCAN_STEP: f64 = 0.04;
const SCAN: f64 = N as f64 * N as f64 * SCAN_STEP;
const HOLD_RESULT: f64 = 1.0;
const FADE_OUT: f64 = 0.6;
const PHASE: f64 = MATRIX_IN + SCAN + HOLD_RESULT + FADE_OUT;
/// Start of the summary section.
const SUMMARY_T: f64 = INTRO + 4.0 * PHASE;
const SUMMARY_FADE: f64 = 0.8;
const SUMMARY_IN: f64 = 0.6;
/// Cell size of the summary thumbnails.
const SMALL_CELL: f64 = 0.115;
const SUMMARY_GRID_STAGGER: f64 = 0.3;
const SUMMARY_PIXEL_DUR: f64 = 0.35;
const SUMMARY_PIXEL_LAG: f64 = 0.006;
/// Extent of one summary grid's lagged fade-in.
const SUMMARY_GRID_IN: f64 = SUMMARY_PIXEL_DUR * (1.0 + (N * N - 1) as f64 * SUMMARY_PIXEL_LAG);
const OUTRO_HOLD: f64 = 2.5;
const TOTAL: f64 =
    SUMMARY_T + SUMMARY_IN + 3.0 * SUMMARY_GRID_STAGGER + SUMMARY_GRID_IN + OUTRO_HOLD;

/// A fade-in at `in_t`, a hold, a fade-out at `out_t` and a hold to `TOTAL`
/// for a group of items (the group's only life cycle).
fn group_seq(
    items: &mut [VItem],
    in_t: f64,
    out_t: f64,
    in_dur: f64,
    out_dur: f64,
) -> AnimSequence {
    let mut s = AnimSequence::new();
    s.forward_to(in_t);
    s.push(
        items
            .iter_mut()
            .map(|it| it.fade_in().with_duration(in_dur))
            .into_lagged(0.02),
    );
    s.hold_to(out_t);
    s.push(
        items
            .iter_mut()
            .map(|it| it.fade_out().with_duration(out_dur))
            .into_lagged(0.0),
    );
    s.hold_to(TOTAL);
    s
}

// MARK: Scan window

/// The sliding 3x3 highlight window: snaps to cell `floor(alpha * n^2)` in
/// raster order over the input grid (custom [`Eval`], like `CubieTurn` in
/// `examples/agents/rubiks_cube`).
struct ScanWindow {
    item: VItem,
    n: usize,
}

impl Eval for ScanWindow {
    type Output = VItem;

    fn eval_alpha(&self, alpha: f64) -> Self::Output {
        let total = self.n * self.n;
        let idx = ((alpha * total as f64).floor() as usize).min(total - 1);
        let (row, col) = (idx / self.n, idx % self.n);
        let mut out = self.item.clone();
        out.move_to(cell_center(INPUT_CENTER, self.n, CELL, row, col));
        out
    }
}

// MARK: Scene

#[scene]
#[wasm_demo_doc]
#[output(dir = "./output/agents/convolution_kernels")]
fn convolution_kernels(r: &mut RanimScene) {
    let mut content = AnimStack::new();

    // Persistent actors: input grid, section labels, kernel matrix frame.
    let mut input_pixels: Vec<VItem> = (0..N * N)
        .map(|i| {
            pixel_square(
                cell_center(INPUT_CENTER, N, CELL, i / N, i % N),
                CELL,
                image_value(i / N, i % N),
            )
        })
        .collect();
    content.push(group_seq(
        &mut input_pixels,
        0.0,
        SUMMARY_T,
        0.5,
        SUMMARY_FADE,
    ));

    for (text, pos) in [
        ("Input", dvec3(INPUT_CENTER.x, 1.75, 0.0)),
        ("Kernel", dvec3(KERNEL_CENTER.x, 1.55, 0.0)),
        ("Output", dvec3(OUTPUT_CENTER.x, 1.75, 0.0)),
    ] {
        let mut label = text_vitems(text, 0.4, pos);
        content.push(group_seq(&mut label, 0.2, SUMMARY_T, 0.6, SUMMARY_FADE));
    }

    let mut matrix_frame: Vec<VItem> = (0..9)
        .map(|i| {
            let sq = Square::new(KCELL * 0.96).with(|sq| {
                sq.center = cell_center(KERNEL_CENTER, 3, KCELL, i / 3, i % 3);
            });
            let mut item = VItem::from(sq);
            item.set_fill_color(rgb8(0x26, 0x26, 0x2e));
            item.set_stroke_color(manim::GREY_B);
            item.set_stroke_width(0.02);
            item
        })
        .collect();
    content.push(group_seq(
        &mut matrix_frame,
        0.4,
        SUMMARY_T,
        0.5,
        SUMMARY_FADE,
    ));

    // The sliding 3x3 window, reused for every kernel.
    let mut window = VItem::from(Square::new(3.0 * CELL + 0.06));
    window.set_fill_opacity(0.0);
    window.set_stroke_color(manim::YELLOW_C);
    window.set_stroke_width(0.05);
    window.move_to(cell_center(INPUT_CENTER, N, CELL, 0, 0));
    let mut window_seq = AnimSequence::new();

    // Per kernel: title, matrix values, output pixels, one scan pass.
    for (k, kernel) in KERNELS.iter().enumerate() {
        let phase_t = INTRO + k as f64 * PHASE;
        let scan_t = phase_t + MATRIX_IN;
        let out_t = scan_t + SCAN + HOLD_RESULT;
        let is_last = k == KERNELS.len() - 1;

        // Title and kernel matrix values.
        let mut title = text_vitems(kernel.name, 0.5, TITLE_POS);
        content.push(group_seq(&mut title, phase_t, out_t, 0.6, FADE_OUT));

        let mut values: Vec<VItem> = (0..9)
            .flat_map(|i| {
                text_vitems(
                    kernel.labels[i / 3][i % 3],
                    0.3,
                    cell_center(KERNEL_CENTER, 3, KCELL, i / 3, i % 3),
                )
            })
            .collect();
        content.push(group_seq(&mut values, phase_t + 0.1, out_t, 0.5, FADE_OUT));

        // Scan pass of the window.
        window_seq.forward_to(scan_t);
        window_seq.push(window.fade_in().with_duration(0.15));
        let scan = ScanWindow {
            item: window.clone(),
            n: N,
        };
        let window_end = scan.eval_alpha(1.0);
        window_seq.push(scan.with_duration(SCAN).with_rate_func(linear));
        window_seq.push(FadeOut::new(window_end).with_duration(0.2));
        window.set_opacity(1.0); // fade_out applied its end state; restore for the next pass

        // Output pixels fade in as the window passes their position.
        let conv = convolve(kernel);
        let (pixel_out_t, pixel_out_dur) = if is_last {
            (SUMMARY_T, SUMMARY_FADE)
        } else {
            (out_t, FADE_OUT)
        };
        for (i, &v) in conv.iter().enumerate() {
            let mut pixel =
                pixel_square(cell_center(OUTPUT_CENTER, N, CELL, i / N, i % N), CELL, v);
            let mut s = AnimSequence::new();
            s.forward_to(scan_t + i as f64 * SCAN_STEP);
            s.push(pixel.fade_in().with_duration(2.0 * SCAN_STEP));
            s.hold_to(pixel_out_t);
            s.push(pixel.fade_out().with_duration(pixel_out_dur));
            s.hold_to(TOTAL);
            content.push(s);
        }
    }
    window_seq.hold_to(TOTAL);
    content.push(window_seq);

    // Summary: the input and all four outputs in a row.
    let mut grids = vec![(
        String::from("Input"),
        (0..N * N)
            .map(|i| image_value(i / N, i % N))
            .collect::<Vec<_>>(),
    )];
    for kernel in KERNELS.iter() {
        grids.push((kernel.name.to_string(), convolve(kernel)));
    }
    for (g, (name, data)) in grids.iter().enumerate() {
        let center = dvec3(-2.0 * 2.72 + g as f64 * 2.72, 0.1, 0.0);
        let in_t = SUMMARY_T + SUMMARY_IN + g as f64 * SUMMARY_GRID_STAGGER;

        let mut pixels: Vec<VItem> = data
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                pixel_square(
                    cell_center(center, N, SMALL_CELL, i / N, i % N),
                    SMALL_CELL,
                    v,
                )
            })
            .collect();
        let mut s = AnimSequence::new();
        s.forward_to(in_t);
        s.push(
            pixels
                .iter_mut()
                .map(|it| it.fade_in().with_duration(SUMMARY_PIXEL_DUR))
                .into_lagged(SUMMARY_PIXEL_LAG),
        );
        s.hold_to(TOTAL);
        content.push(s);

        let mut label = text_vitems(name, 0.3, center + dvec3(0.0, 0.95, 0.0));
        let mut s = AnimSequence::new();
        s.forward_to(in_t);
        s.push(
            label
                .iter_mut()
                .map(|it| it.fade_in().with_duration(0.5))
                .into_lagged(0.05),
        );
        s.hold_to(TOTAL);
        content.push(s);
    }

    r.play(CameraFrame::default().show().with_duration(TOTAL));
    r.play(content);

    // Mid-scan of the Sharpen kernel; the finished summary row.
    r.insert_time_mark(
        INTRO + 2.0 * PHASE + MATRIX_IN + SCAN * 0.55,
        TimeMark::Capture("preview.png".to_string()),
    );
    r.insert_time_mark(TOTAL, TimeMark::Capture("summary.png".to_string()));
}
