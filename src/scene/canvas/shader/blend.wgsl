@group(0) @binding(0) var texture: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    if vertex_index == 0 {
        return vec4<f32>(-1.0, -1.0, 0.0, 1.0);
    } else if vertex_index == 1 || vertex_index == 4 {
        return vec4<f32>(-1.0, 1.0, 0.0, 1.0);
    } else if vertex_index == 2 || vertex_index == 3 {
        return vec4<f32>(1.0, -1.0, 0.0, 1.0);
    } else { // vertex_index == 5
        return vec4<f32>(1.0, 1.0, 0.0, 1.0);
    }
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let tex = textureLoad(texture, vec2<u32>(position.xy), 0);
    return tex;
}
 