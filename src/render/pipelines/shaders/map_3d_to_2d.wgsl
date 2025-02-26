struct CameraUniforms {
    proj_mat: mat4x4<f32>,
    view_mat: mat4x4<f32>,
    half_frame_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> cam_uniforms : CameraUniforms;

// (x, y, z, is_closed)
@group(1) @binding(0) var<storage> points3d: array<vec4<f32>>;
// width
@group(1) @binding(1) var<storage> stroke_width: array<f32>;
// (x, y, is_closed, 0)
@group(1) @binding(2) var<storage, read_write> points2d: array<vec4<f32>>;
struct ClipBox {
    min_x: atomic<i32>,
    max_x: atomic<i32>,
    min_y: atomic<i32>,
    max_y: atomic<i32>,
    max_w: atomic<i32>,
}
@group(1) @binding(3) var<storage, read_write> clip_points: ClipBox;

@compute
@workgroup_size(256)
fn cs_main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let index = global_invocation_id.x;

    var point: vec4<f32>;
    point = cam_uniforms.proj_mat * cam_uniforms.view_mat * vec4(points3d[index].xyz, 1.0);
    let x = point.x / point.w * cam_uniforms.half_frame_size.x;
    let y = point.y / point.w * cam_uniforms.half_frame_size.y;

    atomicMin(&clip_points.min_x, i32(floor(x)));
    atomicMax(&clip_points.max_x, i32(ceil(x)));
    atomicMin(&clip_points.min_y, i32(floor(y)));
    atomicMax(&clip_points.max_y, i32(ceil(y)));
    atomicMax(&clip_points.max_w, i32(ceil(stroke_width[index/2])));

    points2d[index].x = x;
    points2d[index].y = y;
    points2d[index].z = points3d[index].w;
    // points2d[index].z = point.z;
}


