/// Bezier related stuffs
pub mod bezier;
/// Math stuffs
pub mod math;
/// The rate functions
pub mod rate_functions;
// /// Svg related stuffs
// pub mod svg;
// /// Typst related stuffs
// pub mod typst;
// pub(crate) mod wgpu;

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    iter::Sum,
    ops::Div,
};

use glam::{DVec3, Mat3, Vec2, Vec3, vec2, vec3};

/// Projects a 3D point onto a plane defined by a unit normal vector.
pub fn project(p: Vec3, unit_normal: Vec3) -> Vec3 {
    p - unit_normal * unit_normal.dot(p)
}

/// Generate basis vecs for a surface from a unit normal vec
pub fn generate_basis(unit_normal: Vec3) -> (Vec3, Vec3) {
    // trace!("generating basis for {:?}", unit_normal);
    let u = if unit_normal.x != 0.0 || unit_normal.y != 0.0 {
        vec3(-unit_normal.y, unit_normal.x, 0.0)
    } else {
        vec3(1.0, 0.0, 0.0)
    }
    .normalize();
    let v = unit_normal.cross(u).normalize();
    (u, v)
}

/// Get a 3d point's 2d coordinate on a 3d plane
pub fn convert_to_2d(p: Vec3, origin: Vec3, basis: (Vec3, Vec3)) -> Vec2 {
    // trace!("converting {:?} by {:?} and {:?}", p, origin, basis);
    let p_local = p - origin;
    vec2(basis.0.dot(p_local), basis.1.dot(p_local))
}

/// Get a 2d point's 3d coordinate on a 3d plane
pub fn convert_to_3d(p: Vec2, origin: Vec3, basis: (Vec3, Vec3)) -> Vec3 {
    origin + basis.0 * p.x + basis.1 * p.y
}

/// Get a rotation matrix from `v1` to `v2`
pub fn rotation_between_vectors(v1: Vec3, v2: Vec3) -> Mat3 {
    // trace!("rotation_between_vectors: v1: {:?}, v2: {:?}", v1, v2);

    if (v2 - v1).length() < f32::EPSILON {
        return Mat3::IDENTITY;
    }
    let mut axis = v1.cross(v2);
    if axis.length() < f32::EPSILON {
        axis = v1.cross(Vec3::Y);
    }
    if axis.length() < f32::EPSILON {
        axis = v1.cross(Vec3::Z);
    }
    // trace!("axis: {:?}", axis);

    let angle = angle_between_vectors(v1, v2);
    // trace!("angle: {:?}", angle);
    Mat3::from_axis_angle(axis, angle)
}

/// Get data's avg
pub fn avg<T: Clone + Sum + Div<f64, Output = T>>(data: &[T]) -> T {
    data.iter().cloned().sum::<T>() / data.len() as f64
}

/// Get angle between vectors
pub fn angle_between_vectors(v1: Vec3, v2: Vec3) -> f32 {
    if v1.length() == 0.0 || v2.length() == 0.0 {
        return 0.0;
    }

    (v1.dot(v2) / (v1.length() * v2.length()))
        .clamp(-1.0, 1.0)
        .acos()
}

/// Resize the vec while preserving the order
pub fn resize_preserving_order<T: Clone>(vec: &[T], new_len: usize) -> Vec<T> {
    let indices = (0..new_len).map(|i| i * vec.len() / new_len);
    indices.map(|i| vec[i].clone()).collect()
}

/// Resize the vec while preserving the order
///
/// returns the repeated idxs
/// ```
///                     *     *     *     *  repeated
/// [0, 1, 2, 3] -> [0, 0, 1, 1, 2, 2, 3 ,3]
/// ```
pub fn resize_preserving_order_with_repeated_indices<T: Clone>(
    vec: &[T],
    new_len: usize,
) -> (Vec<T>, Vec<usize>) {
    let mut res = Vec::with_capacity(new_len);
    let mut added_idxs = Vec::with_capacity(new_len);
    let mut prev_index = None;
    for i in 0..new_len {
        let index = i * vec.len() / new_len;
        if prev_index.map(|i| i == index).unwrap_or(false) {
            added_idxs.push(res.len());
        }
        res.push(vec[index].clone());
        prev_index = Some(index);
    }
    (res, added_idxs)
}

/// Resize the vec while preserving the order
///
/// returns the repeated cnt of each value
/// ```
///                 [2  2][2  2][2  2][2  2]
/// [0, 1, 2, 3] -> [0, 0, 1, 1, 2, 2, 3 ,3]
/// ```
pub fn resize_preserving_order_with_repeated_cnt<T: Clone>(
    vec: &[T],
    new_len: usize,
) -> (Vec<T>, Vec<usize>) {
    let mut res = Vec::with_capacity(new_len);
    let mut cnts = vec![0; vec.len()];

    let mut src_indices = Vec::with_capacity(new_len);
    for i in 0..new_len {
        let index = i * vec.len() / new_len;
        cnts[index] += 1;
        res.push(vec[index].clone());
        src_indices.push(index);
    }
    (res, src_indices.into_iter().map(|i| cnts[i]).collect())
}

/// Extend the vec with last element
pub fn extend_with_last<T: Clone + Default>(vec: &mut Vec<T>, new_len: usize) {
    let v = vec![vec.last().cloned().unwrap_or_default(); new_len - vec.len()];
    vec.extend(v)
}

// f.a + b.a * (1.0 - f.a)
fn merge_alpha(alpha: f32, n: usize) -> f32 {
    let mut result = alpha;
    for _ in 1..n {
        result = result + (1.0 - result) * alpha;
    }
    result
}

/// Get a target alpha value that can get value of given alpha after mixed n times
pub fn apart_alpha(alpha: f32, n: usize, eps: f32) -> f32 {
    if alpha == 0.0 {
        return 0.0;
    }
    let mut left = (0.0, 0.0);
    let mut right = (1.0, 1.0);

    while right.0 - left.0 > eps {
        let mid_single = (left.0 + right.0) / 2.0;
        let mid_merged = merge_alpha(mid_single, n);

        if (mid_merged - alpha).abs() < f32::EPSILON {
            return mid_single;
        }

        if mid_merged < alpha {
            left = (mid_single, mid_merged);
        } else {
            right = (mid_single, mid_merged);
        }
    }

    ((left.0 + right.0) / 2.0).clamp(0.0, 1.0)
}

/// Calculate hash
pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

/// Apply the function by first transform the points to origin based on a point,
/// then apply the function, then transform the points back.
pub fn wrap_point_func_with_point(
    f: impl Fn(&mut DVec3) + Copy,
    point: DVec3,
) -> impl Fn(&mut DVec3) + Copy {
    move |points| {
        *points -= point;
        f(points);
        *points += point;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_resize_preserve_order_with_repeated_cnt() {
        let values = vec![0, 1, 2, 3];
        let (v, c) = resize_preserving_order_with_repeated_cnt(&values, 8);
        assert_eq!(v, vec![0, 0, 1, 1, 2, 2, 3, 3]);
        assert_eq!(c, vec![2; 8]);
    }

    #[test]
    fn tset_apart_alpha() {
        let a = apart_alpha(1.0, 10, 1e-3);
        println!("{a}");
        println!("{}", merge_alpha(1.0, 10));
    }
}
