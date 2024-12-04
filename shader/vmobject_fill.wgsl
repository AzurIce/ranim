
struct Uniforms {
    matrix: mat4x4<f32>,
    rescale_factors: vec3<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) pos: vec3<f32>,
    @location(1) face: f32,
    @location(2) fill_color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    // @location(1) face: f32,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // out.face = in.face;
    out.position = uniforms.matrix * vec4<f32>(in.pos, 1.0);

    // out.position.x *= uniforms.rescale_factors.x;
    // out.position.y *= uniforms.rescale_factors.y;
    // out.position.z *= uniforms.rescale_factors.z;
    // out.position.w = 1.0 - out.position.z;

    out.color = in.fill_color;
    return out;
}

@fragment
fn fs_main(@builtin(front_facing) front_facing: bool, in: VertexOutput) -> @location(0) vec4<f32> {
    var color = in.color;

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

