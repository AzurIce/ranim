//! Rubik's cube — a full scramble → solve run, shown simultaneously on a 3D
//! cube and on a flat unfolded net.
//!
//! The cube is modelled as 26 cubies, each a single [`MeshItem`] that merges a
//! black body with slightly raised colored sticker quads. A face turn is a
//! custom [`Eval`] (`CubieTurn`) that rotates the transforms of the 9 cubies
//! in the turned layer around the face axis, exactly like `RotateAroundZ` in
//! `examples/tetrahedron_spheres`.
//!
//! The unfolded net (cross layout: `U` on top, `L F R B` in a row, `D` below)
//! is made of 54 [`VItem`] squares placed in a plane facing the camera. The
//! sticker permutation of every move is computed with the same integer 3D
//! rotation math as the cubies, so the net and the 3D cube can never disagree;
//! changed stickers simply morph their fill color near the end of each turn.
//!
//! The solve sequence is the inverse of the scramble: mathematically this is
//! a real solution, and it keeps the example focused on animation rather than
//! on a solver implementation.

use std::f64::consts::{FRAC_PI_2, PI};

use ranim::{
    anims::morph::MorphAnim,
    color::{AlphaColor, Srgb, palettes::manim},
    core::components::rgba::Rgba,
    glam::{DAffine3, DVec3, dvec3},
    items::{
        mesh::MeshItem,
        vitem::{VItem, geometry::Square},
    },
    prelude::*,
    utils::rate_functions::smooth,
};

// MARK: Face model

/// Outward normal of each face, in cubie grid coordinates. Faces are indexed
/// in the order U R F D L B throughout this file.
///
/// The world frame is Z-up; the camera looks at the cube from the
/// (+X, +Y, +Z) octant, so the visible faces are F (+X, green, left on
/// screen), R (+Y, red, right on screen) and U (+Z, white, top).
const FACE_NORMALS: [IVec; 6] = [
    [0, 0, 1],  // U
    [0, 1, 0],  // R
    [1, 0, 0],  // F
    [0, 0, -1], // D
    [0, -1, 0], // L
    [-1, 0, 0], // B
];

/// In-face "right" axis of each face when viewed from outside (net layout).
const FACE_RIGHT: [IVec; 6] = [
    [0, 1, 0],  // U: right points to R
    [-1, 0, 0], // R: right points to B
    [0, 1, 0],  // F: right points to R
    [0, 1, 0],  // D: right points to R
    [1, 0, 0],  // L: right points to F
    [0, -1, 0], // B: right points to L
];

/// In-face "up" axis of each face when viewed from outside.
const FACE_UP: [IVec; 6] = [
    [-1, 0, 0], // U: up points to B
    [0, 0, 1],  // R: up points to U
    [0, 0, 1],  // F: up points to U
    [1, 0, 0],  // D: up points to F
    [0, 0, 1],  // L: up points to U
    [0, 0, 1],  // B: up points to U
];

/// Top-left block offset (column, row) of each face in the net cross layout:
/// `U` above the `L F R B` row, `D` below `F`.
const NET_BLOCKS: [(i32, i32); 6] = [
    (3, 0), // U
    (6, 3), // R
    (3, 3), // F
    (3, 6), // D
    (0, 3), // L
    (9, 3), // B
];

/// Standard cube color scheme (Western): U white, R red, F green, D yellow,
/// L orange, B blue.
const FACE_COLORS: [AlphaColor<Srgb>; 6] = [
    manim::WHITE,    // U
    manim::RED_C,    // R
    manim::GREEN_C,  // F
    manim::YELLOW_C, // D
    manim::ORANGE,   // L
    manim::BLUE_C,   // B
];

/// Integer vector in the cubie grid; components are in `{-1, 0, 1}` for
/// positions and are unit axis vectors for normals.
type IVec = [i32; 3];

fn idot(a: IVec, b: IVec) -> i32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn icross(a: IVec, b: IVec) -> IVec {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn iscale(v: IVec, s: i32) -> IVec {
    [v[0] * s, v[1] * s, v[2] * s]
}

fn iadd(a: IVec, b: IVec) -> IVec {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn to_dvec(v: IVec) -> DVec3 {
    dvec3(v[0] as f64, v[1] as f64, v[2] as f64)
}

/// Rotate `v` by `quarter` quarter-turns around the unit axis vector `axis`
/// (right-hand rule; `quarter` 3 means -90°).
fn rot90(v: IVec, axis: IVec, quarter: i32) -> IVec {
    let parallel = iscale(axis, idot(axis, v));
    match quarter.rem_euclid(4) {
        0 => v,
        // +90°: v' = axis × v + axis (axis · v)  (Rodrigues)
        1 => iadd(icross(axis, v), parallel),
        // 180°: v' = 2 axis (axis · v) - v
        2 => iadd(parallel, iadd(parallel, iscale(v, -1))),
        // -90°: v' = -axis × v + axis (axis · v)
        _ => iadd(iscale(icross(axis, v), -1), parallel),
    }
}

/// Grid position of sticker `idx` (row-major, viewed from outside) of `face`.
fn sticker_pos(face: usize, idx: usize) -> IVec {
    let (row, col) = ((idx / 3) as i32, (idx % 3) as i32);
    iadd(
        FACE_NORMALS[face],
        iadd(
            iscale(FACE_RIGHT[face], col - 1),
            iscale(FACE_UP[face], 1 - row),
        ),
    )
}

/// Inverse of [`sticker_pos`]: locate the (face, idx) of a sticker from its
/// grid position and (rotated) outward normal.
fn sticker_locate(pos: IVec, normal: IVec) -> (usize, usize) {
    let face = FACE_NORMALS
        .iter()
        .position(|&n| n == normal)
        .expect("normal must be a face normal");
    let col = (idot(pos, FACE_RIGHT[face]) + 1) as usize;
    let row = (1 - idot(pos, FACE_UP[face])) as usize;
    (face, row * 3 + col)
}

/// One face turn: `quarter` quarter-turns of the outer layer of `face`
/// (right-hand rule around the outward normal).
#[derive(Clone, Copy)]
struct Move {
    face: usize,
    quarter: i32,
}

impl Move {
    fn inverse(self) -> Self {
        let quarter = match self.quarter {
            1 => 3,
            3 => 1,
            q => q,
        };
        Self {
            face: self.face,
            quarter,
        }
    }

    /// Rotation angle in radians, matching [`rot90`] for the same quarter.
    fn angle(self) -> f64 {
        match self.quarter.rem_euclid(4) {
            1 => FRAC_PI_2,
            2 => PI,
            _ => -FRAC_PI_2,
        }
    }
}

/// Apply the sticker permutation of `mv` to the logical cube state.
fn apply_move(state: &mut [[usize; 9]; 6], mv: Move) {
    let axis = FACE_NORMALS[mv.face];
    let old = *state;
    for (face, old_face) in old.iter().enumerate() {
        for (idx, &color) in old_face.iter().enumerate() {
            let pos = sticker_pos(face, idx);
            if idot(pos, axis) != 1 {
                continue; // sticker not in the turned layer
            }
            let pos = rot90(pos, axis, mv.quarter);
            let normal = rot90(FACE_NORMALS[face], axis, mv.quarter);
            let (f2, i2) = sticker_locate(pos, normal);
            state[f2][i2] = color;
        }
    }
}

/// Deterministic xorshift64-based scramble: `n` moves, never twice the same
/// face in a row.
fn scramble_moves(n: usize, seed: u64) -> Vec<Move> {
    let mut x = seed;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let mut moves = Vec::with_capacity(n);
    while moves.len() < n {
        let face = (next() % 6) as usize;
        if moves.last().is_some_and(|m: &Move| m.face == face) {
            continue;
        }
        let quarter = [1, 2, 3][(next() % 3) as usize];
        moves.push(Move { face, quarter });
    }
    moves
}

// MARK: Cubie meshes

/// Cubie body half-size; the gap between cubies reads as the black frame.
const CUBIE_HALF: f64 = 0.46;
/// Sticker quad half-size.
const STICKER_HALF: f64 = 0.35;
/// How far the sticker quads sit above the cubie body.
const STICKER_LIFT: f64 = 0.005;

/// Append a double-sided quad (4 vertices, 4 triangles) to the mesh buffers.
fn push_quad(
    points: &mut Vec<DVec3>,
    colors: &mut Vec<Rgba>,
    indices: &mut Vec<u32>,
    corners: [DVec3; 4],
    color: Rgba,
) {
    let base = points.len() as u32;
    points.extend(corners);
    colors.extend([color; 4]);
    indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    indices.extend([base + 2, base + 1, base, base + 3, base + 2, base]);
}

/// Build the mesh of the cubie at grid position `grid`: a black body plus one
/// raised colored sticker quad per outward-facing side.
fn cubie_mesh(grid: IVec) -> MeshItem {
    let center = to_dvec(grid);
    let body: Rgba = manim::BLACK.into();

    let mut points = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    // The two in-plane axes for each world axis.
    let in_plane: [(IVec, IVec); 3] = [
        ([0, 1, 0], [0, 0, 1]),
        ([1, 0, 0], [0, 0, 1]),
        ([1, 0, 0], [0, 1, 0]),
    ];

    for axis_i in 0..3 {
        for &sign in &[-1i32, 1] {
            let mut normal = [0; 3];
            normal[axis_i] = sign;
            let n = to_dvec(normal);
            let (e1, e2) = in_plane[axis_i];
            let (e1, e2) = (to_dvec(e1), to_dvec(e2));

            // Body face.
            let c = center + n * CUBIE_HALF;
            push_quad(
                &mut points,
                &mut colors,
                &mut indices,
                [
                    c - e1 * CUBIE_HALF - e2 * CUBIE_HALF,
                    c + e1 * CUBIE_HALF - e2 * CUBIE_HALF,
                    c + e1 * CUBIE_HALF + e2 * CUBIE_HALF,
                    c - e1 * CUBIE_HALF + e2 * CUBIE_HALF,
                ],
                body,
            );

            // Sticker on outward-facing sides.
            if grid[axis_i] == sign {
                let face = FACE_NORMALS.iter().position(|&fn_| fn_ == normal).unwrap();
                let color: Rgba = FACE_COLORS[face].into();
                let c = center + n * (CUBIE_HALF + STICKER_LIFT);
                push_quad(
                    &mut points,
                    &mut colors,
                    &mut indices,
                    [
                        c - e1 * STICKER_HALF - e2 * STICKER_HALF,
                        c + e1 * STICKER_HALF - e2 * STICKER_HALF,
                        c + e1 * STICKER_HALF + e2 * STICKER_HALF,
                        c - e1 * STICKER_HALF + e2 * STICKER_HALF,
                    ],
                    color,
                );
            }
        }
    }

    let mut mesh = MeshItem::from_indexed_vertices(points, indices);
    mesh.vertex_colors = colors.into();
    mesh
}

/// One cubie: its current mesh state, its logical grid position, and its own
/// animation sequence on the shared timeline.
struct Cubie {
    grid: IVec,
    mesh: Transformed<MeshItem>,
    seq: AnimSequence,
}

/// One net sticker square with its own animation sequence.
struct NetSticker {
    item: VItem,
    seq: AnimSequence,
}

/// Rotate a cubie's mesh around `axis` (through the cube center at the
/// origin) by `angle * alpha` — the 3D counterpart of one face turn.
struct CubieTurn {
    src: Transformed<MeshItem>,
    axis: DVec3,
    angle: f64,
}

impl Eval for CubieTurn {
    type Output = Transformed<MeshItem>;

    fn eval_alpha(&self, alpha: f64) -> Self::Output {
        let mut out = self.src.clone();
        out.transform = DAffine3::from_axis_angle(self.axis.normalize(), self.angle * alpha)
            * self.src.transform;
        out
    }
}

// MARK: Timeline

/// Net sticker edge length.
const NET_SIZE: f64 = 0.40;

/// Play one face turn on both the 3D cube and the net, advancing every
/// per-item sequence by `dur` seconds.
fn play_move(
    cubies: &mut [Cubie],
    stickers: &mut [NetSticker],
    state: &mut [[usize; 9]; 6],
    mv: Move,
    dur: f64,
) {
    let axis_ivec = FACE_NORMALS[mv.face];
    let axis = to_dvec(axis_ivec);
    let angle = mv.angle();

    for cubie in cubies.iter_mut() {
        if idot(cubie.grid, axis_ivec) == 1 {
            let anim = CubieTurn {
                src: cubie.mesh.clone(),
                axis,
                angle,
            };
            cubie.seq.push(
                anim.apply_to(&mut cubie.mesh)
                    .with_duration(dur)
                    .with_rate_func(smooth),
            );
            cubie.grid = rot90(cubie.grid, axis_ivec, mv.quarter);
        } else {
            cubie.seq.hold(dur);
        }
    }

    let old = *state;
    apply_move(state, mv);
    for (i, sticker) in stickers.iter_mut().enumerate() {
        let (face, idx) = (i / 9, i % 9);
        if state[face][idx] == old[face][idx] {
            sticker.seq.hold(dur);
        } else {
            // Keep the old color for most of the turn, then flip quickly.
            let color = FACE_COLORS[state[face][idx]];
            sticker.seq.hold(dur * 0.6);
            sticker.seq.push(
                sticker
                    .item
                    .morph(move |it| {
                        it.set_fill_color(color);
                    })
                    .with_duration(dur * 0.4),
            );
        }
    }
}

/// Hold every item for `secs` (pauses between phases).
fn hold_all(cubies: &mut [Cubie], stickers: &mut [NetSticker], secs: f64) {
    for cubie in cubies.iter_mut() {
        cubie.seq.hold(secs);
    }
    for sticker in stickers.iter_mut() {
        sticker.seq.hold(secs);
    }
}

// MARK: Scene

const SCRAMBLE_LEN: usize = 12;
const SCRAMBLE_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

#[scene]
#[wasm_demo_doc]
#[output(dir = "./output/agents/rubiks_cube")]
fn rubiks_cube(r: &mut RanimScene) {
    // Perspective camera in the (+X, +Y, +Z) octant; the look-at target is
    // shifted along the screen-right axis so the cube sits on the left half
    // of the frame and the net on the right half.
    let phi = 62.0_f64.to_radians();
    let theta = 45.0_f64.to_radians();
    let distance = 12.0;
    let mut cam = CameraFrame::from_spherical(phi, theta, distance);
    let screen_right = cam.facing.cross(DVec3::Z).normalize();
    let screen_up = screen_right.cross(cam.facing).normalize();
    let target = screen_right * 2.1;
    cam.set_spherical(phi, theta, distance, target);
    cam.fovy = 0.62;

    // Logical state: state[face][idx] = face index of the sticker's color.
    let mut state: [[usize; 9]; 6] = std::array::from_fn(|f| [f; 9]);

    // 3D cube: 26 cubies centered at the origin.
    let mut cubies: Vec<Cubie> = Vec::with_capacity(26);
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                if x == 0 && y == 0 && z == 0 {
                    continue;
                }
                cubies.push(Cubie {
                    grid: [x, y, z],
                    mesh: Transformed::new(cubie_mesh([x, y, z])),
                    seq: AnimSequence::new(),
                });
            }
        }
    }

    // Flat net in a plane facing the camera, right of the cube.
    let net_center = target + screen_right * 4.2;
    let mut stickers: Vec<NetSticker> = Vec::with_capacity(54);
    for face in 0..6 {
        let (bc, br) = NET_BLOCKS[face];
        for idx in 0..9 {
            let (row, col) = ((idx / 3) as i32, (idx % 3) as i32);
            let x = (bc + col) as f64 * NET_SIZE + NET_SIZE / 2.0 - 6.0 * NET_SIZE;
            let y = 4.5 * NET_SIZE - ((br + row) as f64 * NET_SIZE + NET_SIZE / 2.0);
            let pos = net_center + screen_right * x + screen_up * y;
            let square = Square::new(NET_SIZE * 0.94).with(|sq| {
                sq.axes = (screen_right, screen_up);
                sq.center = pos;
            });
            let mut item = VItem::from(square);
            item.set_fill_color(FACE_COLORS[state[face][idx]]);
            item.set_stroke_color(manim::BLACK);
            item.set_stroke_width(0.03);
            stickers.push(NetSticker {
                item,
                seq: AnimSequence::new(),
            });
        }
    }

    // Intro: show the solved cube.
    let intro = 1.2;
    for cubie in &mut cubies {
        cubie.seq.push(cubie.mesh.show().with_duration(intro));
    }
    for sticker in &mut stickers {
        sticker.seq.push(sticker.item.show().with_duration(intro));
    }
    let mut clock = intro;

    // Scramble.
    let scramble = scramble_moves(SCRAMBLE_LEN, SCRAMBLE_SEED);
    for &mv in &scramble {
        let dur = if mv.quarter == 2 { 0.7 } else { 0.5 };
        play_move(&mut cubies, &mut stickers, &mut state, mv, dur);
        clock += dur;
    }
    let scramble_end = clock;

    // Pause on the scrambled state.
    let pause = 1.0;
    hold_all(&mut cubies, &mut stickers, pause);

    // Solve: inverse of the scramble, slightly faster.
    for mv in scramble.iter().rev().map(|m| m.inverse()) {
        let dur = if mv.quarter == 2 { 0.6 } else { 0.42 };
        play_move(&mut cubies, &mut stickers, &mut state, mv, dur);
    }

    // Outro: hold the solved state.
    let outro = 2.0;
    hold_all(&mut cubies, &mut stickers, outro);

    // Compose everything on one timeline.
    let mut content = AnimStack::new();
    for cubie in cubies {
        content.push(cubie.seq);
    }
    for sticker in stickers {
        content.push(sticker.seq);
    }
    let total_secs = content.duration_secs();

    r.play(cam.show().with_duration(total_secs));
    r.play(content);

    r.insert_time_mark(scramble_end, TimeMark::Capture("preview.png".to_string()));
    r.insert_time_mark(total_secs, TimeMark::Capture("solved.png".to_string()));
}
