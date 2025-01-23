struct CameraUniforms {
    proj_mat: mat4x4<f32>,
    view_mat: mat4x4<f32>,
    half_frame_size: vec2<f32>,
}
@group(0) @binding(0) var<uniform> cam_uniforms : CameraUniforms;

@group(1) @binding(0) var<storage> points: array<vec4<f32>>;
@group(1) @binding(1) var<storage> fill_rgbas: array<vec4<f32>>;
@group(1) @binding(2) var<storage> stroke_rgbas: array<vec4<f32>>;
@group(1) @binding(3) var<storage> stroke_widths: array<f32>;

fn point(idx: u32) -> vec2<f32> {
    return points[idx].xy;
}
fn is_closed(idx: u32) -> bool {
    return bool(points[idx].z);
}

struct SubpathAttr {
    end_idx: u32,
    nearest_idx: u32,
    d: f32, // distance
    sgn: f32,
    debug: vec4<f32>,
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
    ppos = A + (c + b * t.y) + t.y;
    dis = min(dis, length(ppos - pos));
    ppos = A + (c + b * t.z) + t.z;
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
    let bary: vec2<f32> = vec2<f32>(
        c.x * b.y - b.x * c.y,
        a.x * c.y - c.x * a.y
    ) / denominator;

    let d: vec2<f32> = vec2<f32>(bary.y * 0.5, 0.0) + 1.0 - bary.x - bary.y;

    let sign_inside: f32 = select(1.0, sign(d.x * d.x - d.y), d.x > d.y);
    let sign_left: f32 = sign_line(p, A, C);

    return sign_inside * sign_left;
}

// left -> -1.0
// right -> 1.0
fn sign_line(p: vec2<f32>, A: vec2<f32>, B: vec2<f32>) -> f32 {
    let cond: vec3<bool> = vec3(
        p.y >= A.y,
        p.y < B.y,
        cross_2d(B - A, p - A) > 0.0
    );
    return select(1.0, -1.0, all(cond) || !any(cond));
}

fn get_subpath_attr(pos: vec2<f32>, start_idx: u32) -> SubpathAttr {
    var attr: SubpathAttr;
    attr.end_idx = start_idx;
    attr.nearest_idx = 0u;
    attr.d = 3.40282346638528859812e38;
    attr.sgn = 1.0;
    attr.debug = vec4(1.0, 1.0, 1.0, 1.0);

    let points_len = arrayLength(&points);
    let n = (points_len - 1) / 2 * 2;
    for (var i = start_idx; i < n; i += 2u) {
        let a = point(i);
        let b = point(i + 1u);
        let c = point(i + 2u);
        if length(b - a) == 0.0 {
            attr.end_idx = i;
            // attr.debug = vec4(0.0, 0.5, 0.5, 1.0);
            break;
        }

        let v1 = normalize(b - a);
        let v2 = normalize(c - b);

        let is_line = abs(cross_2d(v1, v2)) < 0.0001 && dot(v1, v2) > 0.0;
        // let is_line = true;
        let dist = select(distance_bezier(pos, a, b, c), distance_line(pos, a, c), is_line);
        if dist < attr.d {
            attr.d = dist;
            attr.nearest_idx = i;
        }
        if is_closed(i) {
            attr.sgn *= select(sign_bezier(pos, a, b, c), sign_line(pos, a, c), is_line);
        }
    }

    return attr;
}

fn render(pos: vec2<f32>) -> vec4<f32> {
    let points_len = arrayLength(&points);

    var idx = 0u;
    var d = 3.40282346638528859812e38;
    var sgn = 1.0;

    var start_idx = 0u;
    while start_idx < points_len {
        let attr = get_subpath_attr(pos, start_idx);
        if attr.d < d {
            idx = attr.nearest_idx;
            d = attr.d;
        }
        sgn *= attr.sgn;
        start_idx = attr.end_idx + 2;
    }

    let sgn_d = sgn * d;
    // return vec4(vec3(sgn_d), 1.0);

    let e = point(idx + 1u).xy - point(idx).xy;
    let w = pos.xy - point(idx).xy;
    let ratio = clamp(dot(w, e) / dot(e, e), 0.0, 1.0);
    let anchor_index = idx / 2;

    // TODO: Antialias and clip_box
    var fill_rgba: vec4<f32> = select(vec4(0.0), mix(fill_rgbas[anchor_index], fill_rgbas[anchor_index + 1], ratio), is_closed(idx));
    fill_rgba.a *= smoothstep(1.0, -1.0, (sgn_d));

    let stroke_width = mix(stroke_widths[anchor_index], stroke_widths[anchor_index + 1], ratio);
    var stroke_rgba: vec4<f32> = mix(stroke_rgbas[anchor_index], stroke_rgbas[anchor_index + 1], ratio);
    stroke_rgba.a *= smoothstep(1.0, -1.0, (d - stroke_width));

    var f_color = blend_color(stroke_rgba, fill_rgba);

    return f_color;
}

fn render_control_points(pos: vec2<f32>) -> vec4<f32> {
    let points_len = arrayLength(&points);

    var d = length(pos - point(0u));
    for (var i = 1u; i < points_len; i++) {
        d = min(d, length(pos - point(i)));
    }
    return select(vec4(0.0), vec4(1.0), d < 1);
}

@fragment
fn fs_main(@location(0) pos: vec2<f32>) -> @location(0) vec4<f32> {
    var f_color: vec4<f32>;
    f_color = render(pos);
    // f_color = blend_color(f_color, render_control_points(pos));
    return f_color;
}

struct VertexOutput {
    @builtin(position) frag_pos: vec4<f32>,
    @location(0) pos: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, @location(0) clip_pos: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;

    out.frag_pos = vec4(clip_pos, 0.0, 1.0);
    out.pos = clip_pos * cam_uniforms.half_frame_size;
    return out;
}