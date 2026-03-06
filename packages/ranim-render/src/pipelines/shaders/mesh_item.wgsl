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

struct VertexOutput {
    @builtin(position) frag_pos: vec4<f32>,
    @location(0) @interpolate(flat) mesh_id: u32,
    @location(1) world_pos: vec3<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) world_normal: vec3<f32>,
}

fn pack_color(color: vec4<f32>) -> u32 {
    let c = vec4<u32>(color * 255.0);
    return (c.r) | (c.g << 8u) | (c.b << 16u) | (c.a << 24u);
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
}

fn compute_lighting(world_pos: vec3<f32>, world_normal: vec3<f32>, base_color: vec4<f32>) -> vec4<f32> {
    // Compute both normals unconditionally (derivatives require uniform control flow)
    let flat_normal = normalize(cross(dpdx(world_pos), dpdy(world_pos)));
    let smooth_normal = normalize(world_normal);

    // Select which normal to use based on whether world_normal is near-zero
    let use_flat = dot(world_normal, world_normal) < 0.0001;
    let normal = select(smooth_normal, flat_normal, use_flat);

    let light_dir = normalize(vec3<f32>(0.3, 1.0, 0.5));
    let ambient = 0.35;
    let diffuse = abs(dot(normal, light_dir));
    let lit = ambient + (1.0 - ambient) * diffuse;
    return vec4<f32>(base_color.rgb * lit, base_color.a);
}

@fragment
fn fs_color(
    @builtin(position) frag_pos: vec4<f32>,
    @location(0) @interpolate(flat) mesh_id: u32,
    @location(1) world_pos: vec3<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) world_normal: vec3<f32>,
) -> @location(0) vec4<f32> {
    let color = compute_lighting(world_pos, world_normal, vertex_color);

    // Opaque: output directly
    if (color.a >= 0.99) {
        return color;
    }

    // Transparent: write to OIT buffer, then discard
    let coords = vec2<u32>(floor(frag_pos.xy));
    let pixel_idx = coords.y * frame.x + coords.x;
    let layer_idx = atomicAdd(&pixel_count[pixel_idx], 1u);

    if (layer_idx < frame.z) {
        let buffer_idx = pixel_idx * frame.z + layer_idx;
        oit_colors[buffer_idx] = pack_color(color);
        oit_depths[buffer_idx] = frag_pos.z;
    }

    discard;
    return vec4<f32>(0.0); // To make wasm happy
}

@fragment
fn fs_depth(
    @builtin(position) frag_pos: vec4<f32>,
    @location(0) @interpolate(flat) mesh_id: u32,
    @location(1) world_pos: vec3<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) world_normal: vec3<f32>,
) -> @builtin(frag_depth) f32 {
    let color = vertex_color;

    // Only write depth for opaque objects
    if (color.a < 0.99) {
        discard;
    }

    return frag_pos.z;
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) mesh_id: u32,
    @location(2) vertex_color: vec4<f32>,
    @location(3) vertex_normal: vec3<f32>,
) -> VertexOutput {
    var out: VertexOutput;

    let transform = transforms[mesh_id];
    let pos_world = transform * vec4<f32>(position, 1.0);

    out.frag_pos = cam_uniforms.proj_mat * cam_uniforms.view_mat * pos_world;
    out.mesh_id = mesh_id;
    out.world_pos = pos_world.xyz;
    out.vertex_color = vertex_color;
    out.world_normal = (transform * vec4<f32>(vertex_normal, 0.0)).xyz;

    return out;
}
