struct Plane {
    origin: vec3<f32>,
    basis_u: vec3<f32>,
    basis_v: vec3<f32>,
}

@group(0) @binding(0) var<uniform> plane: Plane;
// (x, y, z, is_closed)
@group(0) @binding(1) var<storage> points3d: array<vec4<f32>>;
// width
@group(0) @binding(2) var<storage> stroke_width: array<f32>;
// (x, y, is_closed, padding)
@group(0) @binding(3) var<storage, read_write> points2d: array<vec4<f32>>;
struct ClipBox {
    min_x: atomic<i32>,
    max_x: atomic<i32>,
    min_y: atomic<i32>,
    max_y: atomic<i32>,
    max_w: atomic<i32>,
}
@group(0) @binding(4) var<storage, read_write> clip_points: ClipBox;
@group(0) @binding(5) var<storage> point_cnt: u32;

@compute
@workgroup_size(256)
fn cs_main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let index = global_invocation_id.x;
    if index >= point_cnt {
        return;
    }

    let p_vec = points3d[index];
    let p = p_vec.xyz;
    let is_closed = p_vec.w;
    let diff = p - plane.origin;

    // Project diff onto the plane spanned by basis_u and basis_v
    // Since basis_u and basis_v are orthogonal and normalized, we can use dot product directly.
    let x = dot(diff, plane.basis_u);
    let y = dot(diff, plane.basis_v);
    let w = stroke_width[index / 2];

    points2d[index] = vec4(x, y, is_closed, 0.0);

    // Note that atomicMin/Max can only work on u32 or i32
    // so we turn the float into int by multiplying by a scale factor
    // and turn it back in the vertex shader
    let scale = 1000.0;
    atomicMin(&clip_points.min_x, i32(floor(x * scale)));
    atomicMax(&clip_points.max_x, i32(ceil(x * scale)));
    atomicMin(&clip_points.min_y, i32(floor(y * scale)));
    atomicMax(&clip_points.max_y, i32(ceil(y * scale)));
    atomicMax(&clip_points.max_w, i32(ceil(w * scale)));
}
