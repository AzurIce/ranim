// === Shared bindings (group 0: resolution/OIT, group 1: camera) ===

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

// === Merged VItem data (group 2) ===

struct ItemInfo {
    point_offset: u32,
    point_count: u32,
    attr_offset: u32,
    attr_count: u32,
}

struct PlaneData {
    origin: vec4<f32>,
    basis_u: vec4<f32>,
    basis_v: vec4<f32>,
}

@group(2) @binding(0) var<storage> item_infos: array<ItemInfo>;
@group(2) @binding(1) var<storage> planes: array<PlaneData>;
// clip_boxes: 5 i32 per item [min_x, max_x, min_y, max_y, max_w]
@group(2) @binding(2) var<storage> clip_boxes: array<i32>;
@group(2) @binding(3) var<storage> points: array<vec4<f32>>;
@group(2) @binding(4) var<storage> fill_rgbas: array<vec4<f32>>;
@group(2) @binding(5) var<storage> stroke_rgbas: array<vec4<f32>>;
@group(2) @binding(6) var<storage> stroke_widths: array<f32>;

// === Per-instance data passed from vertex to fragment ===

struct VertexOutput {
    @builtin(position) frag_pos: vec4<f32>,
    @location(0) pos: vec2<f32>,
    @location(1) @interpolate(flat) instance_id: u32,
}

// === Helper: access item's point/attr data ===

fn item_point(info: ItemInfo, local_idx: u32) -> vec2<f32> {
    return points[info.point_offset + local_idx].xy;
}

fn item_is_closed(info: ItemInfo, local_idx: u32) -> bool {
    return bool(points[info.point_offset + local_idx].z);
}

fn item_fill_rgba(info: ItemInfo, anchor_idx: u32) -> vec4<f32> {
    return fill_rgbas[info.attr_offset + anchor_idx];
}

fn item_stroke_rgba(info: ItemInfo, anchor_idx: u32) -> vec4<f32> {
    return stroke_rgbas[info.attr_offset + anchor_idx];
}

fn item_stroke_width(info: ItemInfo, anchor_idx: u32) -> f32 {
    return stroke_widths[info.attr_offset + anchor_idx];
}

// === SDF math (same as original) ===

fn pack_color(color: vec4<f32>) -> u32 {
    let c = vec4<u32>(color * 255.0);
    return (c.r) | (c.g << 8u) | (c.b << 16u) | (c.a << 24u);
}

fn cross_2d(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return a.x * b.y - a.y * b.x;
}

fn blend_color(f: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    let a = f.a + b.a * (1.0 - f.a);
    return vec4(
        f.r * f.a + b.r * b.a * (1.0 - f.a) / a,
        f.g * f.a + b.g * b.a * (1.0 - f.a) / a,
        f.b * f.a + b.b * b.a * (1.0 - f.a) / a,
        a
    );
}

fn solve_cubic(a: f32, b: f32, c: f32) -> vec3<f32> {
    let p = b - a * a / 3.0;
    let p3 = p * p * p;
    let q = a * (2.0 * a * a - 9.0 * b) / 27.0 + c;
    let d = q * q + 4.0 * p3 / 27.0;
    let offset = -a / 3.0;
    if (d >= 0.0) {
        let z = sqrt(d);
        let x = (vec2(z, -z) - q) / 2.0;
        let uv = sign(x) * pow(abs(x), vec2(1.0 / 3.0));
        return vec3(offset + uv.x + uv.y);
    }
    let v = acos(-sqrt(-27.0 / p3) * q / 2.0) / 3.0;
    let m = cos(v);
    let n = sin(v) * 1.732050808;
    return vec3(m + m, -n - m, n - m) * sqrt(-p / 3.0) + offset;
}

fn distance_bezier(pos: vec2<f32>, A: vec2<f32>, _B: vec2<f32>, C: vec2<f32>) -> f32 {
    var B = mix(_B + vec2(1e-4), _B, abs(sign(_B * 2.0 - A - C)));
    let a = B - A;
    let b = A - B * 2.0 + C;
    let c = a * 2.0;
    let d = A - pos;
    let k = vec3(3.0 * dot(a, b), 2.0 * dot(a, a) + dot(d, b), dot(d, a)) / dot(b, b);
    let solved = solve_cubic(k.x, k.y, k.z);
    let t = vec3(
        clamp(solved.x, 0.0, 1.0),
        clamp(solved.y, 0.0, 1.0),
        clamp(solved.z, 0.0, 1.0),
    );
    var ppos = A + (c + b * t.x) * t.x;
    var dis = length(ppos - pos);
    ppos = A + (c + b * t.y) * t.y;
    dis = min(dis, length(ppos - pos));
    ppos = A + (c + b * t.z) * t.z;
    dis = min(dis, length(ppos - pos));
    return dis;
}

fn distance_line(pos: vec2<f32>, A: vec2<f32>, B: vec2<f32>) -> f32 {
    let e = B - A;
    let w = pos - A;
    let b = w - e * clamp(dot(w, e) / dot(e, e), 0.0, 1.0);
    return length(b);
}

fn sign_bezier(p: vec2<f32>, A: vec2<f32>, B: vec2<f32>, C: vec2<f32>) -> f32 {
    let a: vec2<f32> = C - A;
    let b: vec2<f32> = B - A;
    let c: vec2<f32> = p - A;
    let denominator: f32 = a.x * b.y - b.x * a.y;
    let bary: vec2<f32> = vec2<f32>(cross_2d(c, b), cross_2d(a, c)) / denominator;
    let d: vec2<f32> = vec2<f32>(bary.y * 0.5, 0.0) + 1.0 - bary.x - bary.y;
    let sign_inside: f32 = select(1.0, sign(d.x * d.x - d.y), d.x > d.y);
    let sign_left: f32 = sign_line(p, A, C);
    return sign_inside * sign_left;
}

fn sign_line(p: vec2<f32>, A: vec2<f32>, B: vec2<f32>) -> f32 {
    let cond: vec3<bool> = vec3(
        (p.y >= A.y),
        (p.y < B.y),
        (cross_2d(B - A, p - A) > 0.0)
    );
    return select(1.0, -1.0, all(cond) || !any(cond));
}

// === SDF rendering (adapted for merged buffers) ===

struct SubpathAttr {
    end_idx: u32,
    nearest_idx: u32,
    d: f32,
    sgn: f32,
}

fn get_subpath_attr(pos: vec2<f32>, info: ItemInfo, start_local_idx: u32) -> SubpathAttr {
    var attr: SubpathAttr;
    attr.end_idx = info.point_count;
    attr.nearest_idx = 0u;
    attr.d = 3.40282346638528859812e38;
    attr.sgn = 1.0;

    let n = (info.point_count - 1u) / 2u * 2u;
    for (var i = start_local_idx; i < n; i += 2u) {
        let a = item_point(info, i);
        let b = item_point(info, i + 1u);
        let c = item_point(info, i + 2u);
        if length(b - a) == 0.0 {
            attr.end_idx = i;
            break;
        }

        let v1 = normalize(b - a);
        let v2 = normalize(c - b);
        let is_line = abs(cross_2d(v1, v2)) < 0.0001 && dot(v1, v2) > 0.0;
        let dist = select(distance_bezier(pos, a, b, c), distance_line(pos, a, c), is_line);
        if dist < attr.d {
            attr.d = dist;
            attr.nearest_idx = i;
        }
        if item_is_closed(info, i) {
            attr.sgn *= select(sign_bezier(pos, a, b, c), sign_line(pos, a, c), is_line);
        }
    }

    return attr;
}

fn render(pos: vec2<f32>, info: ItemInfo) -> vec4<f32> {
    var idx = 0u;
    var d = 3.40282346638528859812e38;
    var sgn = 1.0;

    var start_idx = 0u;
    while start_idx < info.point_count {
        let attr = get_subpath_attr(pos, info, start_idx);
        if attr.d < d {
            idx = attr.nearest_idx;
            d = attr.d;
        }
        sgn *= attr.sgn;
        start_idx = attr.end_idx + 2u;
    }

    let sgn_d = sgn * d;

    let e = item_point(info, idx + 1u) - item_point(info, idx);
    let w = pos - item_point(info, idx);
    let ratio = clamp(dot(w, e) / dot(e, e), 0.0, 1.0);
    let anchor_index = idx / 2u;

    let antialias_radius = 0.015 / 4.0;

    var fill_rgba: vec4<f32> = select(
        vec4(0.0),
        mix(item_fill_rgba(info, anchor_index), item_fill_rgba(info, anchor_index + 1u), ratio),
        item_is_closed(info, idx)
    );
    fill_rgba.a *= smoothstep(1.0, -1.0, (sgn_d) / antialias_radius);

    var stroke_width = mix(
        item_stroke_width(info, anchor_index),
        item_stroke_width(info, anchor_index + 1u),
        ratio
    );
    var stroke_rgba: vec4<f32> = mix(
        item_stroke_rgba(info, anchor_index),
        item_stroke_rgba(info, anchor_index + 1u),
        ratio
    );
    stroke_rgba.a *= smoothstep(1.0, -1.0, (d - stroke_width) / antialias_radius);

    var f_color = blend_color(stroke_rgba, fill_rgba);

    if (f_color.a < 0.01) {
        discard;
    }

    return f_color;
}

// === Fragment shaders ===

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

@fragment
fn fs_main(
    @builtin(position) frag_pos: vec4<f32>,
    @location(0) pos: vec2<f32>,
    @location(1) @interpolate(flat) instance_id: u32,
) -> FragmentOutput {
    var out: FragmentOutput;
    let info = item_infos[instance_id];
    let color = render(pos, info);

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
    @location(0) pos: vec2<f32>,
    @location(1) @interpolate(flat) instance_id: u32,
) -> @builtin(frag_depth) f32 {
    let info = item_infos[instance_id];
    let color = render(pos, info);

    if (color.a < 0.99) {
        discard;
    }

    return frag_pos.z;
}

// === Vertex shader ===

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let info = item_infos[instance_index];
    let plane = planes[instance_index];
    let clip_base = instance_index * 5u;

    let scale = 1000.0;
    let min_x = f32(clip_boxes[clip_base + 0u]) / scale;
    let max_x = f32(clip_boxes[clip_base + 1u]) / scale;
    let min_y = f32(clip_boxes[clip_base + 2u]) / scale;
    let max_y = f32(clip_boxes[clip_base + 3u]) / scale;
    let max_w = f32(clip_boxes[clip_base + 4u]) / scale;

    var clip_point: vec2<f32>;
    clip_point.x = select(
        max_x + max_w,
        min_x - max_w,
        (vertex_index & 2u) == 0u
    );
    clip_point.y = select(
        max_y + max_w,
        min_y - max_w,
        (vertex_index & 1u) == 0u
    );

    let u = clip_point.x;
    let v = clip_point.y;

    let pos3d = plane.origin.xyz + u * plane.basis_u.xyz + v * plane.basis_v.xyz;

    out.frag_pos = cam_uniforms.proj_mat * cam_uniforms.view_mat * vec4(pos3d, 1.0);
    out.pos = clip_point;
    out.instance_id = instance_index;
    return out;
}
