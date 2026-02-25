@group(0) @binding(0) var depth_tex: texture_depth_2d;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    let pos = vec2<f32>(uv * 2.0 - 1.0);
    return vec4<f32>(pos.x, -pos.y, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let coords = vec2<i32>(floor(frag_pos.xy));
    let depth = textureLoad(depth_tex, coords, 0);
    return vec4<f32>(vec3<f32>(depth), 1.0);
}
