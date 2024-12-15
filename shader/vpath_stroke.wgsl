
struct Uniforms {
    matrix: mat4x4<f32>,
    rescale_factors: vec3<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var<storage> vertices: array<Vertex>;

struct Vertex {
    pos: vec4<f32>,
    color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // let pos = vec2(f32(vertex_index) / 80.0, 0.0);
    var in = vertices[vertex_index];

    var out: VertexOutput;
    // out.position = vec4<f32>(pos, 0.0, 1.0);
    // if vertex_index % 3 == 0 {
    //     out.position = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    // } else if vertex_index % 3 == 1 {
    //     out.position = vec4<f32>(-0.5, 0.0, 0.0, 1.0);
    // } else {
    //     out.position = vec4<f32>(-0.5, -0.5, 0.0, 1.0);
    // }
    out.position = uniforms.matrix * in.pos;
    out.position.y *= -1.0;

    // out.position.x *= uniforms.rescale_factors.x;
    // out.position.y *= uniforms.rescale_factors.y;
    // out.position.z *= uniforms.rescale_factors.z;
    // out.position.w = 1.0 - out.position.z;

    out.color = in.color;
    // out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    // var inpos = uniforms.matrix * in.pos;
    // inpos.x *= uniforms.rescale_factors.x;
    // inpos.y *= uniforms.rescale_factors.y;
    // inpos.z *= uniforms.rescale_factors.z;

    // out.color = abs(inpos);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

