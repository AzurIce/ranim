struct CameraUniforms {
    proj_mat: mat4x4<f32>,
    view_mat: mat4x4<f32>,
    half_frame_size: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> cam_uniforms: CameraUniforms;

struct NVPoint {
    points: array<vec4<f32>, 3>,
    closepath: vec4<f32>,
}

// 64 bytes
@group(1) @binding(0)
var<storage> input_data: array<NVPoint>;
// width
@group(1) @binding(1)
var<storage> stroke_width: array<f32>;

// 64 bytes
@group(1) @binding(2)
var<storage, read_write> output_data: array<NVPoint>;
struct ClipBox {
    min_x: atomic<i32>,
    max_x: atomic<i32>,
    min_y: atomic<i32>,
    max_y: atomic<i32>,
    max_w: atomic<i32>,
}

@group(1) @binding(3)
var<storage, read_write> points_len: u32;

@group(1) @binding(4)
var<storage, read_write> clip_points: ClipBox;

@compute @workgroup_size(256)
fn cs_main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let index = global_invocation_id.x;

    if index >= points_len {
        return;
    }

    for (var i = 0; i < 3; i++) {
        let p = cam_uniforms.proj_mat * cam_uniforms.view_mat * vec4(input_data[index].points[i].xyz, 1.0);

        let x = p.x / p.w * cam_uniforms.half_frame_size.x;
        let y = p.y / p.w * cam_uniforms.half_frame_size.y;
        let z = p.z / p.w;

        output_data[index].points[i].x = x;
        output_data[index].points[i].y = y;
        output_data[index].points[i].z = z;
        output_data[index].points[i].w = 1.0;

        atomicMin(&clip_points.min_x, i32(floor(x)));
        atomicMax(&clip_points.max_x, i32(ceil(x)));
        atomicMin(&clip_points.min_y, i32(floor(y)));
        atomicMax(&clip_points.max_y, i32(ceil(y)));
    }

    output_data[index].closepath = input_data[index].closepath;
    atomicMax(&clip_points.max_w, i32(ceil(stroke_width[index])));
}


