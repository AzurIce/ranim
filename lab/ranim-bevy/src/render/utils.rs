use bevy::prelude::*;
use ranim_core::glam::Vec3 as RanimVec3;

pub(crate) fn vitem_normal_from_points(points: &[Vec4]) -> Vec3 {
    if points.len() < 3 {
        return Vec3::Z;
    }
    let p0 = points[0].truncate();
    let p1 = points[1].truncate();
    let p2 = points[2].truncate();
    let normal = (p1 - p0).cross(p2 - p0);
    if normal.length_squared() < 1e-6 {
        Vec3::Z
    } else {
        normal.normalize()
    }
}

pub(crate) fn basis_from_normal(normal: RanimVec3) -> (RanimVec3, RanimVec3) {
    let arbitrary = if normal.x.abs() > 0.99 {
        RanimVec3::Y
    } else {
        RanimVec3::X
    };
    let basis_u = normal.cross(arbitrary).normalize();
    let basis_v = normal.cross(basis_u);
    (basis_u, basis_v)
}

pub(crate) fn resize_vec4_by_sample(values: &[Vec4], target_len: usize) -> Vec<Vec4> {
    if target_len == 0 {
        return Vec::new();
    }
    if values.is_empty() {
        return vec![Vec4::ZERO; target_len];
    }
    if values.len() == target_len {
        return values.to_vec();
    }

    let step = values.len() as f32 / target_len as f32;
    (0..target_len)
        .map(|idx| {
            let source_idx = (idx as f32 * step).floor() as usize;
            values[source_idx.min(values.len() - 1)]
        })
        .collect()
}

pub(crate) fn resize_f32_by_sample(values: &[f32], target_len: usize) -> Vec<f32> {
    if target_len == 0 {
        return Vec::new();
    }
    if values.is_empty() {
        return vec![0.0; target_len];
    }
    if values.len() == target_len {
        return values.to_vec();
    }

    let step = values.len() as f32 / target_len as f32;
    (0..target_len)
        .map(|idx| {
            let source_idx = (idx as f32 * step).floor() as usize;
            values[source_idx.min(values.len() - 1)]
        })
        .collect()
}
