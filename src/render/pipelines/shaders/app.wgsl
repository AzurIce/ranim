@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> viewport: Viewport;

struct Viewport {
    width: f32,
    height: f32,
    x: f32,
    y: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index)index: u32) -> VertexOutput {
    var out: VertexOutput;
    let width = viewport.width;
    let height = viewport.height;
    var x = viewport.x;
    var y = viewport.y;

    // 1.0, 1.0, -1.0, -1.0
    let coord_x = select(1.0, -1.0, (index & 2u) == 0u);
    // 1.0, -1.0, 1.0, -1.0
    let coord_y = select(1.0, -1.0, (index & 1u) == 0u);
    x += coord_x * width;
    y += coord_y * height;
    
    out.position = vec4<f32>(
        x,
        y,
        0.0,
        1.0,
    );
    out.tex_coords = vec2<f32>((coord_x + 1.0) / 2.0, (1 - coord_y) / 2.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // return vec4(1.0, 0.0, 0.0, 1.0);
    return textureSample(texture, texture_sampler, in.tex_coords);
}


