
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
    @location(1) face: f32,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.face = in.face;
    out.position = uniforms.matrix * vec4<f32>(in.pos, 1.0);

    out.position.x *= uniforms.rescale_factors.x;
    out.position.y *= uniforms.rescale_factors.y;
    out.position.z *= uniforms.rescale_factors.z;
    // out.position.w = 1.0 - out.position.z;

    out.color = in.fill_color;
    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    // @builtin(frag_depth) depth: f32,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    var color = in.color;
    color.a *= 0.95;

    // out.depth = in.position.z;
    out.color = color;
    if in.face < 0.0 {
        out.color.a = -color.a / (1.0 - color.a);
        // out.color.a = 0.0;
        // out.color.a = -color.a;
        // out.color = vec4<f32>(1.0, 0.0, 0.0, 0.1);
        // out.depth = 1.0;
    } else {
        // out.color = vec4<f32>(0.0, 1.0, 0.0, 0.1);
    }
    return out;
}

