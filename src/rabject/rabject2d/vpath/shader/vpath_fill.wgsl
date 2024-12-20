
struct Uniforms {
    matrix: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) pos: vec3<f32>,
    @location(1) fill_type: u32,
    @location(2) fill_color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) fill_type: u32,
    @location(2) uv_coord: vec2<f32>,
}

// https://www.dafaycoding.com/article/android-basic-bezier
const SIMPLE_CUBIC: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(0.0, 1.0),
    vec2<f32>(0.552, 1.0),
    vec2<f32>(1.0, 0.552),
    vec2<f32>(1.0, 0.0),
);

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // out.face = in.face;
    out.position = uniforms.matrix * vec4<f32>(in.pos, 1.0);
    out.position.y *= -1.0;
    if in.fill_type == 1 {
        if in.vertex_index % 3 == 0 {
            out.uv_coord = SIMPLE_CUBIC[0];
        } else if in.vertex_index % 3 == 1 {
            out.uv_coord = SIMPLE_CUBIC[1];
        } else if in.vertex_index % 3 == 2 {
            out.uv_coord = SIMPLE_CUBIC[2];
        }
    } else if in.fill_type == 2 {
        if in.vertex_index % 3 == 0 {
            out.uv_coord = SIMPLE_CUBIC[0];
        } else if in.vertex_index % 3 == 1 {
            out.uv_coord = SIMPLE_CUBIC[2];
        } else if in.vertex_index % 3 == 2 {
            out.uv_coord = SIMPLE_CUBIC[3];
        }
    }

    // out.position.x *= uniforms.rescale_factors.x;
    // out.position.y *= uniforms.rescale_factors.y;
    // out.position.z *= uniforms.rescale_factors.z;
    // out.position.w = 1.0 - out.position.z;

    out.color = in.fill_color;
    out.fill_type = in.fill_type;
    return out;
}

@fragment
fn fs_main(@builtin(front_facing) front_facing: bool, in: VertexOutput) -> @location(0) vec4<f32> {
    var color = in.color;

    if in.fill_type == 0 {
        return color;
    }

    let x = in.uv_coord.x;
    let y = in.uv_coord.y;
    // let Fxy = y - x * x;
    let Fxy = y * y + x * x;
    if Fxy > 1.0 {
        discard;
    }
    // if front_facing {
    //     color = vec4<f32>(1.0, 0.0, 0.0, 0.1);
    // } else {
    //     color = vec4<f32>(0.0, 1.0, 0.0, 0.1);
    // }
    // var color = in.color;
    // color.a *= 0.95;

    // if in.face < 0.0 {
        // color.a = -color.a / (1.0 - color.a);
        // color.a = 0.0;
        // color.a = -color.a;
        // color = vec4<f32>(1.0, 0.0, 0.0, 0.1);
    // } else {
        // color = vec4<f32>(0.0, 1.0, 0.0, 0.1);
    // }
    return color;
}

