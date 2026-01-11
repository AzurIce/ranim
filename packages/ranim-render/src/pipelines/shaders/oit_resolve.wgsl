struct CameraUniforms {
    proj_mat: mat4x4<f32>,
    view_mat: mat4x4<f32>,
    half_frame_size: vec2<f32>,
    resolution: vec2<u32>,
    oit_layers: u32,
}
@group(0) @binding(0) var<uniform> cam_uniforms : CameraUniforms;

@group(0) @binding(1) var<storage, read_write> pixel_count: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> oit_colors: array<u32>;
@group(0) @binding(3) var<storage, read_write> oit_depths: array<f32>;

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

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

@fragment
fn fs_main(@builtin(position) frag_pos: vec4<f32>) -> FragmentOutput {
    let coords = vec2<i32>(floor(frag_pos.xy));

    // Start with fully transparent
    var final_color = vec4<f32>(0.0);

    let pixel_idx = u32(coords.y) * cam_uniforms.resolution.x + u32(coords.x);
    let count_atomic = atomicLoad(&pixel_count[pixel_idx]);

    // Clear the counter for the next frame (IMPORTANT! This acts as the clear pass for the atomic buffer)
    // NOTE: In a real multi-pass architecture, clearing might be done in a separate Compute Pass or by the API.
    // But since this is a read-modify-write structure, we need to reset it.
    // However, atomicExchange here is racy if we are rendering *while* resolving (which shouldn't happen due to barriers/pass structure).
    // A safer way is a separate clear pass, but for simplicity in this specific setup if we assume sequential execution:
    // atomicStore(&pixel_count[pixel_idx], 0u);
    // Actually, doing it here prevents us from using the data for anything else later and implies read-after-write hazard if not careful.
    // It's better to clear it at the start of the frame.
    // We will assume a separate Clear logic exists or `pixel_count` is cleared before the OIT recording pass.

    let count = min(count_atomic, cam_uniforms.oit_layers);

    if (count == 0u) {
        discard;
    }

    const MAX_LAYERS: u32 = 16u;
    var nodes: array<Node, MAX_LAYERS>;

    let loops = min(count, MAX_LAYERS);

    for (var i = 0u; i < loops; i++) {
        let buffer_idx = pixel_idx * cam_uniforms.oit_layers + i;
        nodes[i].color = unpack_color(oit_colors[buffer_idx]);
        nodes[i].depth = oit_depths[buffer_idx];
    }

    for (var i = 0u; i < loops; i++) {
        for (var j = i + 1u; j < loops; j++) {
            if (nodes[i].depth < nodes[j].depth) {
                let temp = nodes[i];
                nodes[i] = nodes[j];
                nodes[j] = temp;
            }
        }
    }

    // Blend
    for (var i = 0u; i < loops; i++) {
        // Simple alpha blend: src OVER dst
        // final_color is 'dst' (background), node is 'src' (foreground layer)
        let src = nodes[i].color;
        let dst = final_color;

        // Standard alpha blending:
        // out_a = src_a + dst_a * (1 - src_a)
        // out_rgb = (src_rgb * src_a + dst_rgb * dst_a * (1 - src_a)) / out_a

        // Optimised accumulation:
        // We can just accumulate normally.
        final_color = blend(src, dst);
    }

    var output: FragmentOutput;

    output.color = final_color;
    output.depth = nodes[loops - 1].depth;
    return output;
}
