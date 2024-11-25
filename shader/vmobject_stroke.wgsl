
struct Uniforms {
    matrix: mat4x4<f32>,
    rescale_factors: vec3<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) pos: vec4<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = uniforms.matrix * in.pos;

    out.position.x *= uniforms.rescale_factors.x;
    out.position.y *= uniforms.rescale_factors.y;
    out.position.z *= uniforms.rescale_factors.z;
    // out.position.w = 1.0 - out.position.z;

    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

