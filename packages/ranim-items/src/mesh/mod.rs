//! Mesh-based items (Surface, Sphere, etc.)

use ranim_core::glam::DVec3;

mod sphere;
mod surface;

pub use sphere::*;
pub use surface::*;

/// Compute smooth vertex normals from a triangle mesh.
///
/// Each face normal is weighted by the angle at the vertex before accumulation.
/// The result is normalized per vertex. Degenerate triangles are skipped.
pub fn compute_smooth_normals(points: &[DVec3], triangle_indices: &[u32]) -> Vec<DVec3> {
    let mut normals = vec![DVec3::ZERO; points.len()];

    for tri in triangle_indices.chunks_exact(3) {
        let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        let (p0, p1, p2) = (points[i0], points[i1], points[i2]);

        let e01 = p1 - p0;
        let e02 = p2 - p0;
        let face_normal = e01.cross(e02);

        // Skip degenerate triangles
        if face_normal.length_squared() < 1e-20 {
            continue;
        }

        // Weight by angle at each vertex
        let e10 = p0 - p1;
        let e12 = p2 - p1;
        let e20 = p0 - p2;
        let e21 = p1 - p2;

        let angle0 = angle_between(e01, e02);
        let angle1 = angle_between(e10, e12);
        let angle2 = angle_between(e20, e21);

        normals[i0] += face_normal * angle0;
        normals[i1] += face_normal * angle1;
        normals[i2] += face_normal * angle2;
    }

    for n in &mut normals {
        let len = n.length();
        if len > 1e-10 {
            *n /= len;
        }
    }

    normals
}

/// Angle (in radians) between two vectors.
fn angle_between(a: DVec3, b: DVec3) -> f64 {
    let denom = a.length() * b.length();
    if denom < 1e-20 {
        return 0.0;
    }
    (a.dot(b) / denom).clamp(-1.0, 1.0).acos()
}

/// Generate triangle indices for a `nu × nv` grid of vertices (row-major layout).
///
/// Each quad `[i, j]` → 2 triangles: `[tl, bl, tr]` and `[tr, bl, br]`
/// where `tl = i*nv + j`, `tr = i*nv + j+1`, `bl = (i+1)*nv + j`, `br = (i+1)*nv + j+1`.
///
/// Total index count = `6 * (nu - 1) * (nv - 1)`.
pub fn generate_grid_indices(nu: u32, nv: u32) -> Vec<u32> {
    let mut indices = Vec::with_capacity(6 * (nu as usize - 1) * (nv as usize - 1));
    for i in 0..nu - 1 {
        for j in 0..nv - 1 {
            let tl = i * nv + j;
            let tr = i * nv + j + 1;
            let bl = (i + 1) * nv + j;
            let br = (i + 1) * nv + j + 1;
            // Triangle 1: tl, bl, tr
            indices.push(tl);
            indices.push(bl);
            indices.push(tr);
            // Triangle 2: tr, bl, br
            indices.push(tr);
            indices.push(bl);
            indices.push(br);
        }
    }
    indices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_grid_indices_2x2() {
        // 2×2 grid → 1 quad → 2 triangles → 6 indices
        let indices = generate_grid_indices(2, 2);
        assert_eq!(indices.len(), 6);
        // Vertices: 0=tl, 1=tr, 2=bl, 3=br
        assert_eq!(indices, vec![0, 2, 1, 1, 2, 3]);
    }

    #[test]
    fn test_generate_grid_indices_3x3() {
        // 3×3 grid → 4 quads → 8 triangles → 24 indices
        let indices = generate_grid_indices(3, 3);
        assert_eq!(indices.len(), 24);
    }

    #[test]
    fn test_generate_grid_indices_count() {
        let nu = 10;
        let nv = 5;
        let indices = generate_grid_indices(nu, nv);
        assert_eq!(indices.len(), 6 * (nu as usize - 1) * (nv as usize - 1));
    }
}
