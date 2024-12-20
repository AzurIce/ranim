
struct Uniforms {
    matrix: mat4x4<f32>,
    rescale_factors: vec3<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var canvas_texture: texture_2d<f32>;
@group(1) @binding(1) var canvas_texture_sampler: sampler;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = uniforms.matrix * vec4<f32>(in.position / 2.0, 1.0);
    out.uv = in.uv;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // return textureSample(canvas_texture, canvas_texture_sampler, in.uv);
    // return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    return vec4<f32>(in.uv, 0.0, 1.0);
}

