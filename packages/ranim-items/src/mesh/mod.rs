//! Mesh-based items (Surface, Sphere, etc.)

mod sphere;
mod surface;

pub use sphere::*;
pub use surface::*;

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
