use bevy::{asset::uuid_handle, prelude::*};

pub(crate) const RANIM_VITEM_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("1c7031bb-904d-47c2-9b79-30e6f3fe0a91");

pub(crate) const RANIM_VITEM_SHADER: &str = r#"
#import bevy_pbr::mesh_view_bindings::view

#ifdef OIT_ENABLED
#import bevy_core_pipeline::oit::oit_draw
#endif

struct ItemInfo {
    point_offset: u32,
    point_count: u32,
    attr_offset: u32,
    attr_count: u32,
}

struct PlaneData {
    normal: vec4<f32>,
    origin: vec4<f32>,
}

struct InstanceInfo {
    world_from_local: mat4x4<f32>,
    item_index: u32,
    _padding: array<u32, 3>,
}

@group(2) @binding(0) var<storage> item_infos: array<ItemInfo>;
@group(2) @binding(1) var<storage> planes: array<PlaneData>;
@group(2) @binding(2) var<storage> points: array<vec4<f32>>;
@group(2) @binding(3) var<storage> fill_rgbas: array<vec4<f32>>;
@group(2) @binding(4) var<storage> stroke_rgbas: array<vec4<f32>>;
@group(2) @binding(5) var<storage> stroke_widths: array<f32>;
@group(2) @binding(6) var<storage> instances: array<InstanceInfo>;

struct Basis {
    u: vec3<f32>,
    v: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) @interpolate(flat) item_index: u32,
}

fn basis_from_normal(n: vec3<f32>) -> Basis {
    let arbitrary = select(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), abs(n.x) > 0.99);
    let basis_u = normalize(cross(n, arbitrary));
    let basis_v = cross(n, basis_u);
    return Basis(basis_u, basis_v);
}

fn world_point_for_plane(local_pos: vec2<f32>, instance_index: u32) -> vec3<f32> {
    let instance = instances[instance_index];
    let plane = planes[instance.item_index];
    let basis = basis_from_normal(plane.normal.xyz);
    let local_plane_point = plane.origin.xyz + basis.u * local_pos.x + basis.v * local_pos.y;
    return (instance.world_from_local * vec4<f32>(local_plane_point, 1.0)).xyz;
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let item_index = instances[instance_index].item_index;
    let info = item_infos[item_index];
    var min_xy = vec2<f32>(3.402823e38);
    var max_xy = vec2<f32>(-3.402823e38);
    var stroke_max = 0.0;

    for (var i = 0u; i < info.point_count; i = i + 1u) {
        let point = points[info.point_offset + i];
        min_xy = min(min_xy, point.xy);
        max_xy = max(max_xy, point.xy);
    }
    for (var i = 0u; i < info.attr_count; i = i + 1u) {
        stroke_max = max(stroke_max, stroke_widths[info.attr_offset + i]);
    }

    let pad = max(stroke_max * 2.0, 0.08);
    min_xy -= vec2<f32>(pad);
    max_xy += vec2<f32>(pad);

    let corner = array<vec2<f32>, 4>(
        vec2<f32>(min_xy.x, min_xy.y),
        vec2<f32>(max_xy.x, min_xy.y),
        vec2<f32>(min_xy.x, max_xy.y),
        vec2<f32>(max_xy.x, max_xy.y),
    );

    var out: VertexOutput;
    out.local_pos = corner[vertex_index];
    out.item_index = item_index;
    out.clip_position = view.clip_from_world * vec4<f32>(world_point_for_plane(out.local_pos, instance_index), 1.0);
    return out;
}

fn item_point(info: ItemInfo, local_idx: u32) -> vec2<f32> {
    return points[info.point_offset + local_idx].xy;
}

fn item_is_closed(info: ItemInfo, local_idx: u32) -> bool {
    return bool(points[info.point_offset + local_idx].z);
}

fn item_fill_rgba(info: ItemInfo, anchor_idx: u32) -> vec4<f32> {
    return fill_rgbas[info.attr_offset + min(anchor_idx, info.attr_count - 1u)];
}

fn item_stroke_rgba(info: ItemInfo, anchor_idx: u32) -> vec4<f32> {
    return stroke_rgbas[info.attr_offset + min(anchor_idx, info.attr_count - 1u)];
}

fn item_stroke_width(info: ItemInfo, anchor_idx: u32) -> f32 {
    return stroke_widths[info.attr_offset + min(anchor_idx, info.attr_count - 1u)];
}

fn cross_2d(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return a.x * b.y - a.y * b.x;
}

fn blend_color(f: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    let a = f.a + b.a * (1.0 - f.a);
    if a <= 0.0 {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(
        (f.rgb * f.a + b.rgb * b.a * (1.0 - f.a)) / a,
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
        let x = (vec2<f32>(z, -z) - q) / 2.0;
        let uv = sign(x) * pow(abs(x), vec2<f32>(1.0 / 3.0));
        return vec3<f32>(offset + uv.x + uv.y);
    }
    let v = acos(-sqrt(-27.0 / p3) * q / 2.0) / 3.0;
    let m = cos(v);
    let n = sin(v) * 1.732050808;
    return vec3<f32>(m + m, -n - m, n - m) * sqrt(-p / 3.0) + offset;
}

fn distance_bezier(pos: vec2<f32>, A: vec2<f32>, _B: vec2<f32>, C: vec2<f32>) -> f32 {
    var B = mix(_B + vec2<f32>(1e-4), _B, abs(sign(_B * 2.0 - A - C)));
    let a = B - A;
    let b = A - B * 2.0 + C;
    let c = a * 2.0;
    let d = A - pos;
    let k = vec3<f32>(3.0 * dot(a, b), 2.0 * dot(a, a) + dot(d, b), dot(d, a)) / dot(b, b);
    let solved = solve_cubic(k.x, k.y, k.z);
    let t = vec3<f32>(
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

fn sign_line(p: vec2<f32>, A: vec2<f32>, B: vec2<f32>) -> f32 {
    let cond = vec3<bool>(
        (p.y >= A.y),
        (p.y < B.y),
        (cross_2d(B - A, p - A) > 0.0),
    );
    return select(1.0, -1.0, all(cond) || !any(cond));
}

fn sign_bezier(p: vec2<f32>, A: vec2<f32>, B: vec2<f32>, C: vec2<f32>) -> f32 {
    let a = C - A;
    let b = B - A;
    let c = p - A;
    let denominator = a.x * b.y - b.x * a.y;
    let bary = vec2<f32>(cross_2d(c, b), cross_2d(a, c)) / denominator;
    let d = vec2<f32>(bary.y * 0.5, 0.0) + 1.0 - bary.x - bary.y;
    let sign_inside = select(1.0, sign(d.x * d.x - d.y), d.x > d.y);
    let sign_left = sign_line(p, A, C);
    return sign_inside * sign_left;
}

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
    attr.d = 3.402823e38;
    attr.sgn = 1.0;

    let n = (info.point_count - 1u) / 2u * 2u;
    for (var i = start_local_idx; i < n; i = i + 2u) {
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

fn render_vitem(pos: vec2<f32>, info: ItemInfo) -> vec4<f32> {
    var idx = 0u;
    var d = 3.402823e38;
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

    var fill_rgba = select(
        vec4<f32>(0.0),
        mix(item_fill_rgba(info, anchor_index), item_fill_rgba(info, anchor_index + 1u), ratio),
        item_is_closed(info, idx)
    );
    fill_rgba.a *= smoothstep(1.0, -1.0, sgn_d / antialias_radius);

    var stroke_width = mix(
        item_stroke_width(info, anchor_index),
        item_stroke_width(info, anchor_index + 1u),
        ratio
    );
    var stroke_rgba = mix(
        item_stroke_rgba(info, anchor_index),
        item_stroke_rgba(info, anchor_index + 1u),
        ratio
    );
    stroke_rgba.a *= smoothstep(1.0, -1.0, (d - stroke_width) / antialias_radius);

    return blend_color(stroke_rgba, fill_rgba);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let info = item_infos[in.item_index];
    let color = render_vitem(in.local_pos, info);
    if color.a < 0.01 {
        discard;
    }
#ifdef OIT_ENABLED
    oit_draw(in.clip_position, color);
    discard;
#endif
    return color;
}
"#;
