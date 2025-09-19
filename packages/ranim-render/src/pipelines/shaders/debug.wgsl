struct VertexInput {
    @builtin(vertex_index) index: u32,
    @location(0) data: vec4<f32>
}
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) data: vec4<f32>
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(in.index % (960 / 2)) / 960.0 * 2.0;
    let y = f32(in.index / (960 / 2)) / 360.0 * 2.0;
    out.pos = vec4(f32(x), f32(y), 0.0, 1.0);
    out.data = (in.data + 1.0) / 2.0;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // return vec4(1.0, 0.0, 0.0, 1.0);
    return vec4(in.data.xyz, 1.0);
}