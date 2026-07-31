//! 3D cloth simulation: a spring-net trampoline catching a falling ball.
//!
//! Iterative-only: spring forces, self-collision and ball-cloth collision have
//! no closed form; all state (particle positions, ball position/velocity) is
//! carried across frames by `step`.
//!
//! The cloth is a grid of particles with its four corners pinned. A ball drops
//! from above; the stiff spring net holds its shape under its own weight, dips
//! elastically under the ball, bounces it a couple of times, and settles with
//! the ball resting in the sag. The cloth is rendered as a shaded `MeshItem`
//! surface (smooth normals computed each frame); the ball is a static sphere
//! mesh moved by its transform.

use ranim::{
    color::palettes::manim,
    core::{
        animation::{Evaluator, SegmentTime},
        components::rgba::Rgba,
    },
    glam::{DVec3, Mat4, Vec3, dvec3},
    items::mesh::{MeshItem, Sphere, Surface},
    prelude::*,
};

const ROWS: usize = 16;
const COLS: usize = 16;
const SPACING: f64 = 0.22;
const CLOTH_Y: f64 = 2.2;
// Stiff springs keep the net from sagging noticeably under its own weight
// (~0.2 units vs ~0.6 before); moderate gravity and damping give the ball a
// couple of lively bounces before it settles.
const GRAVITY: f64 = 0.8;
const DAMPING: f64 = 0.985;
const K_STRUCTURAL: f64 = 1200.0;
const K_SHEAR: f64 = 800.0;
const K_BEND: f64 = 400.0;
const REPULSION_CUTOFF: f64 = 0.4 * SPACING;
const REPULSION_K: f64 = 300.0;
const BALL_RADIUS: f64 = 0.5;
const BALL_START: [f64; 3] = [0.0, 3.5, 0.0];
/// How strongly the cloth pushes the ball back during a collision (1.0 = full).
const BALL_COLLISION_FACTOR: f64 = 1.0;
const TOTAL_SECS: f64 = 12.0;

/// A spring-net cloth with a falling ball.
struct TrampolineCloth {
    curr: Vec<DVec3>,
    prev: Vec<DVec3>,
    pinned: Vec<bool>,
    springs: Vec<(usize, usize, f64, f64)>, // (i, j, rest_len, stiffness)
    initial: Vec<DVec3>,
    ball_curr: DVec3,
    ball_prev: DVec3,
    ball_mesh: MeshItem,
    cloth_indices: Vec<u32>,
}

impl TrampolineCloth {
    fn new() -> Self {
        let width = (COLS - 1) as f64 * SPACING;
        let mut curr = Vec::with_capacity(ROWS * COLS);
        for r in 0..ROWS {
            for c in 0..COLS {
                curr.push(dvec3(
                    -width / 2.0 + c as f64 * SPACING,
                    CLOTH_Y,
                    -width / 2.0 + r as f64 * SPACING,
                ));
            }
        }
        // Pin the four corners.
        let pinned = (0..ROWS * COLS)
            .map(|i| {
                let (r, c) = (i / COLS, i % COLS);
                (r == 0 || r == ROWS - 1) && (c == 0 || c == COLS - 1)
            })
            .collect();

        let mut springs = Vec::new();
        for r in 0..ROWS {
            for c in 0..COLS {
                let i = r * COLS + c;
                if c + 1 < COLS {
                    springs.push((i, i + 1, SPACING, K_STRUCTURAL));
                }
                if r + 1 < ROWS {
                    springs.push((i, i + COLS, SPACING, K_STRUCTURAL));
                }
                if c + 1 < COLS && r + 1 < ROWS {
                    springs.push((i, i + COLS + 1, SPACING * 2.0f64.sqrt(), K_SHEAR));
                }
                if c + 2 < COLS {
                    springs.push((i, i + 2, 2.0 * SPACING, K_BEND));
                }
                if r + 2 < ROWS {
                    springs.push((i, i + 2 * COLS, 2.0 * SPACING, K_BEND));
                }
            }
        }

        // Grid quads -> two triangles each, wound for +Y normals on the flat cloth.
        let mut cloth_indices = Vec::with_capacity(6 * (ROWS - 1) * (COLS - 1));
        for r in 0..ROWS - 1 {
            for c in 0..COLS - 1 {
                let a = (r * COLS + c) as u32;
                let b = (r * COLS + c + 1) as u32;
                let d = ((r + 1) * COLS + c) as u32;
                let e = ((r + 1) * COLS + c + 1) as u32;
                cloth_indices.extend_from_slice(&[a, e, b, a, d, e]);
            }
        }

        let ball_mesh = MeshItem::from(Surface::from(
            Sphere::new(BALL_RADIUS)
                .with_resolution((20, 12))
                .with_fill_color(manim::RED_C),
        ));

        Self {
            initial: curr.clone(),
            prev: curr.clone(),
            curr,
            pinned,
            springs,
            ball_curr: dvec3(BALL_START[0], BALL_START[1], BALL_START[2]),
            ball_prev: dvec3(BALL_START[0], BALL_START[1], BALL_START[2]),
            ball_mesh,
            cloth_indices,
        }
    }
}

impl Evaluator for TrampolineCloth {
    type Output = Vec<MeshItem>;

    fn reset(&mut self) {
        self.curr = self.initial.clone();
        self.prev = self.initial.clone();
        self.ball_curr = dvec3(BALL_START[0], BALL_START[1], BALL_START[2]);
        self.ball_prev = dvec3(BALL_START[0], BALL_START[1], BALL_START[2]);
    }

    fn step(&mut self, time: &SegmentTime) {
        let dt2 = time.local_delta_secs * time.local_delta_secs;
        let n = self.curr.len();

        // Ball: Verlet integration with gravity.
        let bv = (self.ball_curr - self.ball_prev) * DAMPING;
        self.ball_prev = self.ball_curr;
        self.ball_curr += bv - dvec3(0.0, GRAVITY, 0.0) * dt2;

        // Cloth spring forces.
        let mut ax = vec![0.0f64; n];
        let mut ay = vec![0.0f64; n];
        let mut az = vec![0.0f64; n];
        for &(i, j, rest, k) in &self.springs {
            let d = self.curr[j] - self.curr[i];
            let dist = d.length();
            if dist < 1e-9 {
                continue;
            }
            let f = k * (dist - rest) / dist;
            ax[i] += f * d.x;
            ay[i] += f * d.y;
            az[i] += f * d.z;
            ax[j] -= f * d.x;
            ay[j] -= f * d.y;
            az[j] -= f * d.z;
        }

        // Self-collision repulsion (keeps the net from folding onto itself).
        for i in 0..n {
            for j in (i + 1)..n {
                let d = self.curr[j] - self.curr[i];
                let dist = d.length();
                if dist < REPULSION_CUTOFF && dist > 1e-9 {
                    let f = REPULSION_K * (REPULSION_CUTOFF - dist) / dist;
                    ax[i] -= f * d.x;
                    ay[i] -= f * d.y;
                    az[i] -= f * d.z;
                    ax[j] += f * d.x;
                    ay[j] += f * d.y;
                    az[j] += f * d.z;
                }
            }
        }

        // Cloth Verlet integration.
        for i in 0..n {
            if self.pinned[i] {
                continue;
            }
            let vx = (self.curr[i].x - self.prev[i].x) * DAMPING;
            let vy = (self.curr[i].y - self.prev[i].y) * DAMPING;
            let vz = (self.curr[i].z - self.prev[i].z) * DAMPING;
            self.prev[i] = self.curr[i];
            self.curr[i].x += vx + ax[i] * dt2;
            self.curr[i].y += vy + (ay[i] - GRAVITY) * dt2;
            self.curr[i].z += vz + az[i] * dt2;
        }

        // Ball-cloth collision: push cloth particles out of the ball and push
        // the ball back (leaving `ball_prev` untouched gives a natural bounce).
        for i in 0..n {
            if self.pinned[i] {
                continue;
            }
            let d = self.curr[i] - self.ball_curr;
            let dist = d.length();
            if dist < BALL_RADIUS && dist > 1e-9 {
                let normal = d / dist;
                let penetration = BALL_RADIUS - dist;
                self.curr[i] += normal * penetration;
                self.ball_curr -= normal * (penetration * BALL_COLLISION_FACTOR);
            }
        }
    }

    fn sample(&self, _time: &SegmentTime) -> Vec<MeshItem> {
        let points: Vec<Vec3> = self.curr.iter().map(|p| p.as_vec3()).collect();

        // Smooth normals: accumulate face normals, then normalize.
        let mut normals = vec![Vec3::ZERO; points.len()];
        for [a, b, c] in self.cloth_indices.as_chunks::<3>().0 {
            let (a, b, c) = (*a as usize, *b as usize, *c as usize);
            let face = (points[b] - points[a]).cross(points[c] - points[a]);
            normals[a] += face;
            normals[b] += face;
            normals[c] += face;
        }
        for n in &mut normals {
            *n = n.normalize_or_zero();
        }

        let vertex_colors = vec![Rgba::from(manim::WHITE.with_alpha(1.0)); points.len()];
        let cloth = MeshItem {
            points: points.into(),
            triangle_indices: self.cloth_indices.clone(),
            transform: Mat4::IDENTITY,
            vertex_colors: vertex_colors.into(),
            vertex_normals: normals.into(),
        };

        let mut ball = self.ball_mesh.clone();
        ball.transform = Mat4::from_translation(self.ball_curr.as_vec3());

        vec![cloth, ball]
    }
}

#[scene]
#[output(dir = "./output/cloth_trampoline")]
fn cloth_trampoline(r: &mut RanimScene) {
    let mut camera = CameraFrame::default();
    camera.pos = dvec3(-4.0, 3.2, 4.0);
    camera.facing = (dvec3(0.0, 1.8, 0.0) - camera.pos).normalize();
    camera.up = DVec3::Y;
    camera.perspective_blend = 1.0;
    camera.fovy = 50.0f64.to_radians();

    r.play(camera.show().with_duration(TOTAL_SECS));
    r.play(TrampolineCloth::new().with_duration(TOTAL_SECS));
}
