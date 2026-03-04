@group(0) @binding(0) var<uniform> frame: vec3<u32>;
@group(0) @binding(1) var<storage, read_write> pixel_count: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> oit_colors: array<u32>;
@group(0) @binding(3) var<storage, read_write> oit_depths: array<f32>;

struct CameraUniforms {
    proj_mat: mat4x4<f32>,
    view_mat: mat4x4<f32>,
    half_frame_size: vec2<f32>,
}
@group(1) @binding(0) var<uniform> cam_uniforms: CameraUniforms;

@group(2) @binding(0) var<storage> transforms: array<mat4x4<f32>>;
@group(2) @binding(1) var<storage> fill_rgbas: array<vec4<f32>>;

struct VertexOutput {
    @builtin(position) frag_pos: vec4<f32>,
    @location(0) @interpolate(flat) mesh_id: u32,
}

fn pack_color(color: vec4<f32>) -> u32 {
    let c = vec4<u32>(color * 255.0);
    return (c.r) | (c.g << 8u) | (c.b << 16u) | (c.a << 24u);
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

@fragment
fn fs_main(
    @builtin(position) frag_pos: vec4<f32>,
    @location(0) @interpolate(flat) mesh_id: u32,
) -> FragmentOutput {
    var out: FragmentOutput;
    let color = fill_rgbas[mesh_id];

    if (color.a >= 0.99) {
        out.color = color;
        out.depth = frag_pos.z;
        return out;
    }

    let coords = vec2<u32>(floor(frag_pos.xy));
    let pixel_idx = coords.y * frame.x + coords.x;
    let layer_idx = atomicAdd(&pixel_count[pixel_idx], 1u);

    if (layer_idx < frame.z) {
        let buffer_idx = pixel_idx * frame.z + layer_idx;
        oit_colors[buffer_idx] = pack_color(color);
        oit_depths[buffer_idx] = frag_pos.z;
    }

    discard;
    out.color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    out.depth = 1.0;
    return out;
}

@fragment
fn fs_depth_only(
    @builtin(position) frag_pos: vec4<f32>,
    @location(0) @interpolate(flat) mesh_id: u32,
) -> @builtin(frag_depth) f32 {
    let color = fill_rgbas[mesh_id];

    if (color.a < 0.99) {
        discard;
    }

    return frag_pos.z;
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) mesh_id: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let transform = transforms[mesh_id];
    let pos_world = transform * vec4<f32>(position, 1.0);

    out.frag_pos = cam_uniforms.proj_mat * cam_uniforms.view_mat * pos_world;
    out.mesh_id = mesh_id;

    return out;
}
