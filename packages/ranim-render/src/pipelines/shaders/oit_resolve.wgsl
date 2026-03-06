@group(0) @binding(0) var<uniform> frame: vec3<u32>;
@group(0) @binding(1) var<storage, read_write> pixel_count: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> oit_colors: array<u32>;
@group(0) @binding(3) var<storage, read_write> oit_depths: array<f32>;

@group(1) @binding(0) var depth_texture: texture_depth_2d;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    // Generate a full-screen triangle:
    // (0, 0), (2, 0), (0, 2) covers (-1, -1) to (3, 3) in clip space
    // effectively covering the [-1, 1] range.
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    out.uv = uv;
    out.position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    // Invert Y for WGSL clip space if needed (often handled by the API, but standard triangle assumes bottom-left origin for UV 0,0 usually, clip space is Y up)
    // Actually, simple full screen quad is often:
    // (-1, -1), (3, -1), (-1, 3)
    out.position.y = -out.position.y;
    return out;
}

struct Node {
    color: vec4<f32>,
    depth: f32,
}

fn unpack_color(packed: u32) -> vec4<f32> {
    let r = f32(packed & 0xFFu) / 255.0;
    let g = f32((packed >> 8u) & 0xFFu) / 255.0;
    let b = f32((packed >> 16u) & 0xFFu) / 255.0;
    let a = f32((packed >> 24u) & 0xFFu) / 255.0;
    return vec4<f32>(r, g, b, a);
}

// Simple blend function (standard alpha blending: src OVER dst)
fn blend(src: vec4<f32>, dst: vec4<f32>) -> vec4<f32> {
    // result.a = src.a + dst.a * (1.0 - src.a);
    // result.rgb = (src.rgb * src.a + dst.rgb * dst.a * (1.0 - src.a)) / result.a;
    // However, since we are doing compositing, we can simplify assuming premultiplied alpha
    // or standard interpolation.
    // Standard non-premultiplied alpha blending:
    let out_a = src.a + dst.a * (1.0 - src.a);
    if (out_a <= 0.0) { return vec4(0.0); }
    let out_rgb = (src.rgb * src.a + dst.rgb * dst.a * (1.0 - src.a)) / out_a;
    return vec4(out_rgb, out_a);
}

@fragment
fn fs_main(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let coords = vec2<i32>(floor(frag_pos.xy));

    // Start with fully transparent
    var final_color = vec4<f32>(0.0);

    let pixel_idx = u32(coords.y) * frame.x + u32(coords.x);
    let count_atomic = atomicLoad(&pixel_count[pixel_idx]);
    let count = min(count_atomic, frame.z);

    if (count == 0u) {
        discard;
    }

    // Sample opaque depth
    let opaque_depth = textureLoad(depth_texture, vec2<u32>(coords), 0);

    const MAX_LAYERS: u32 = 16u;
    var nodes: array<Node, MAX_LAYERS>;

    let loops = min(count, MAX_LAYERS);

    // Load layers and filter by depth
    var valid_count = 0u;
    for (var i = 0u; i < loops; i++) {
        let buffer_idx = pixel_idx * frame.z + i;
        let layer_depth = oit_depths[buffer_idx];

        // Skip layers behind opaque surfaces
        if (layer_depth > opaque_depth) {
            continue;
        }

        nodes[valid_count].color = unpack_color(oit_colors[buffer_idx]);
        nodes[valid_count].depth = layer_depth;
        valid_count++;
    }

    if (valid_count == 0u) {
        discard;
    }

    // Sort valid layers by depth (back to front)
    for (var i = 0u; i < valid_count; i++) {
        for (var j = i + 1u; j < valid_count; j++) {
            if (nodes[i].depth < nodes[j].depth) {
                let temp = nodes[i];
                nodes[i] = nodes[j];
                nodes[j] = temp;
            }
        }
    }

    // Blend valid layers
    for (var i = 0u; i < valid_count; i++) {
        let src = nodes[i].color;
        let dst = final_color;
        final_color = blend(src, dst);
    }

    return final_color;
}
