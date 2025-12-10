// (x, y, is_closed, padding)
@group(0) @binding(0) var<storage> points2d: array<vec4<f32>>;
// width
@group(0) @binding(1) var<storage> stroke_width: array<f32>;
struct ClipBox {
    min_x: atomic<i32>,
    max_x: atomic<i32>,
    min_y: atomic<i32>,
    max_y: atomic<i32>,
    max_w: atomic<i32>,
}
@group(0) @binding(2) var<storage, read_write> clip_points: ClipBox;
@group(0) @binding(3) var<storage> point_cnt: u32;

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

    let point = points2d[index];
    let x = point.x;
    let y = point.y;
    let w = stroke_width[index / 2];
    // z is is_closed, w is padding

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