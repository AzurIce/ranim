@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index)index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = select(1.0, -1.0, (index & 2u) == 0u);
    let y = select(1.0, -1.0, (index & 1u) == 0u);
    out.position = vec4<f32>(
        x,
        y,
        0.0,
        1.0,
    );
    out.tex_coords = vec2<f32>((x + 1.0) / 2.0, (1 - y) / 2.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // return vec4(1.0, 0.0, 0.0, 1.0);
    return textureSample(texture, texture_sampler, in.tex_coords);
}


