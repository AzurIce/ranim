struct CameraUniforms {
    proj_mat: mat4x4<f32>,
    view_mat: mat4x4<f32>,
    half_frame_size: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> cam_uniforms: CameraUniforms;

struct NVPoint {
    prev_handle: vec4<f32>,
    anchor: vec4<f32>,
    next_handle: vec4<f32>,
    closepath: vec4<f32>,
}

@group(1) @binding(0)
var<storage> points: array<NVPoint>;
@group(1) @binding(1)
var<storage> fill_rgbas: array<vec4<f32>>;
@group(1) @binding(2)
var<storage> stroke_rgbas: array<vec4<f32>>;
@group(1) @binding(3)
var<storage> stroke_widths: array<f32>;
struct ClipBox {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    max_w: i32,
}

@group(1) @binding(4)
var<storage> clip_box: ClipBox;

fn point(idx: u32) -> vec2<f32> {
    return points[idx].anchor.xy;
}

fn prev_handle(idx: u32) -> vec2<f32> {
    return points[idx].prev_handle.xy;
}

fn next_handle(idx: u32) -> vec2<f32> {
    return points[idx].next_handle.xy;
}

fn is_closed(idx: u32) -> bool {
    return bool(points[idx].closepath[0]);
}

struct SubpathAttr {
    end_idx: u32,
    nearest_idx: u32,
    d: f32,
    // distance
    sgn: f32,
    debug: vec4<f32>,
}

fn cross_2d(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return a.x * b.y - a.y * b.x;
}

fn blend_color(f: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    let a = f.a + b.a * (1.0 - f.a);
    return vec4(f.r * f.a + b.r * b.a * (1.0 - f.a) / a, f.g * f.a + b.g * b.a * (1.0 - f.a) / a, f.b * f.a + b.b * b.a * (1.0 - f.a) / a, a);
}

struct SolveCubicRes {
    root: array<f32, 3>,
    debug: vec4<f32>,
}

fn solve_cubic(a_3: f32, b_3: f32, c: f32) -> SolveCubicRes {
    let a = a_3 * 3.0;
    let b = b_3 * 3.0;

    let p = b - a * a_3;
    let p3 = p * p * p;

    let q = a * (a * a / 13.5 - b_3) + c;
    let d = q * q + p3 / 6.75;
    let offset = - a_3;

    var res: SolveCubicRes;

    // single_root
    if (d >= 0.0) {
        let z = sqrt(d);
        let x = (vec2(z, - z) - q) / 2.0;
        let uv = sign(x) * pow(abs(x), vec2(1.0 / 3.0));

        var r = offset + uv.x + uv.y;

        let f = ((r + a) * r + b) * r + c;
        let f_prime = (3.0 * r + 2.0 * a) * r + b;

        r -= f / f_prime;
        res.root = array(r, r, r);
        res.debug = vec4(0.0, 0.0, 1.0, 1.0); // Blue for single root
        return res;
    }
    let u = sqrt(- p / 3.0);
    let v = acos(clamp(- sqrt(- 27.0 / p3) * q / 2.0, - 1.0, 1.0)) / 3.0;
    let m = cos(v);
    let n = sin(v) * 1.732050808;

    var r = vec3(m + m, - n - m, n - m) * u + offset;
    var r_array = array(r.x, r.y, r.z);

    for (var i = 0u; i < 3u; i++) {
        for (var j = 0u; j < 6u; j++) {
            let f = ((r_array[i] + a) * r_array[i] + b) * r_array[i] + c;
            let f_prime = (3.0 * r_array[i] + 2.0 * a) * r_array[i] + b;

            if abs(f_prime) < 1e-6 {
                break;
            }

            let delta = f / f_prime;
            r_array[i] -= delta;

            if length(delta) < 1e-6 {
                break;
            }
        }
    }

    res.root = r_array;
    res.debug = vec4(0.0, 1.0, 0.0, 1.0); // Green for three roots
    return res;
}

// Implemented from https://www.shadertoy.com/view/3lsSzS
const num_iterations: u32 = 3;
const num_start_params: u32 = 3;
const factor: f32 = 1.0;

fn cubic_bezier_iteration(t: f32, A0: vec2<f32>, A1: vec2<f32>, A2: vec2<f32>, A3: vec2<f32>) -> f32 {
    let a2 = A2 + A3 * t;
    let a1 = A1 + a2 * t;
    let b2 = a2 + A3 * t;

    let p = A0 + a1 * t;
    let tan = a1 + b2 * t;

    let l_tan = dot(tan, tan);

    return t - factor * dot(tan, p) / l_tan;
}

fn distance_bezier_sq(pos: vec2<f32>, p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p3: vec2<f32>) -> f32 {
    let a0 = p0 - pos;
    let a1 = (- 3.0 * p0 + 3.0 * p1);
    let a2 = (3.0 * p0 - 6.0 * p1 + 3.0 * p2);
    let a3 = (- p0 + 3.0 * p1 - 3.0 * p2 + p3);

    var d0 = 1e38;
    var t0 = 0.0;
    var t = 0.0;
    for (var i = 0u; i < num_start_params; i++) {
        t = t0;
        for (var j = 0u; j < num_iterations; j++) {
            t = cubic_bezier_iteration(t, a0, a1, a2, a3);
        }
        t = clamp(t, 0.0, 1.0);
        let p = ((a3 * t + a2) * t + a1) * t + a0;
        d0 = min(d0, dot(p, p));
        t0 += 1.0 / f32(num_start_params - 1);
    }
    return d0;
}

fn sign_root(root: f32, pos: vec2<f32>, p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p3: vec2<f32>) -> f32 {
    let x_pos = ((((- p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x) * root + (3.0 * p0.x - 6.0 * p1.x + 3.0 * p2.x)) * root) + (- 3.0 * p0.x + 3.0 * p1.x)) * root + p0.x;
    return select(1.0, - 1.0, x_pos < pos.x && root >= 0.0 && root <= 1.0);
}

fn eval_bezier(t: f32, p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p3: vec2<f32>) -> vec2<f32> {
    return ((p3 * t + p2) * t + p1) * t + p0;
}

fn tan_bezier(t: f32, p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p3: vec2<f32>) -> vec2<f32> {
    // The tangent of a cubic Bezier curve is the derivative of the curve equation
    // B'(t) = 3(1-t)²(p1-p0) + 6(1-t)t(p2-p1) + 3t²(p3-p2)
    let t1 = 1.0 - t;
    let term1 = 3.0 * t1 * t1 * (p1 - p0);
    let term2 = 6.0 * t1 * t * (p2 - p1);
    let term3 = 3.0 * t * t * (p3 - p2);
    return term1 + term2 + term3;
}

struct SignBezierRes {
    sgn: f32,
    debug_color: vec4<f32>,
};

fn sign_bezier(pos: vec2<f32>, p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p3: vec2<f32>) -> SignBezierRes {
    // Coeffecient of the equation `cubic bezier.y = pos.y`
    let cu = (p3.y + p1.y * 3.0) - (p2.y * 3.0 + p0.y);
    let qu_3 = p0.y - 2.0 * p1.y + p2.y;
    let li_3 = - p0.y + p1.y;
    let co = p0.y - pos.y;

    var res: SignBezierRes;
    res.sgn = 1.0;
    res.debug_color = vec4(0.0, 0.0, 0.0, 1.0);
    // quadratic
    // For example, when:
    //
    //  + handle1    + handle2
    //
    //  + anchor1    + anchor2
    // The equation degenerate to a quadratic equation
    if abs(cu) < 1e-6 {
        res.debug_color = vec4(1.0, 0.0, 0.0, 1.0); // Red for quadratic

        let d = 9.0 * li_3 * li_3 - 12.0 * qu_3 * co;
        if abs(d) < 1e-6 {
            let root = - li_3 / (2.0 * qu_3);
            // This is a workaround to fix the case where the tangent on the root is horizontal
            if tan_bezier(root, p0, p1, p2, p3).y != 0.0 {
                res.sgn *= sign_root(root, pos, p0, p1, p2, p3);
            }
        } else if d > 0.0 {
            let root1 = (- li_3 - sqrt(d) / 3.0) / (2.0 * qu_3);
            res.sgn *= sign_root(root1, pos, p0, p1, p2, p3);
            let root2 = (- li_3 + sqrt(d) / 3.0) / (2.0 * qu_3);
            res.sgn *= sign_root(root2, pos, p0, p1, p2, p3);
        }
    }
    else {
        let res_cubic = solve_cubic(qu_3 / cu, li_3 / cu, co / cu);
        for (var i = 0u; i < 3u; i++) {
            res.sgn *= sign_root(res_cubic.root[i], pos, p0, p1, p2, p3);
        }
        res.debug_color = res_cubic.debug;
    }

    // // let tan1 = p0.xy - p1.xy;
    // // let tan2 = p2.xy - p3.xy;
    // // let nor1 = vec2(tan1.y, - tan1.x);
    // // let nor2 = vec2(tan2.y, - tan2.x);

    // // let cond1: vec3<bool> = vec3(p0.y < p1.y, pos.y <= p0.y, dot(pos - p0.xy, nor1) < 0.0);
    // // sgn *= select(1.0, - 1.0, all(cond1) || !any(cond1));

    // // let cond2: vec3<bool> = vec3(p2.y < p3.y, pos.y> p3.y, dot(pos - p3.xy, nor2) < 0.0);
    // // sgn *= select(1.0, - 1.0, all(cond2) || !any(cond2));
    return res;
}

fn get_subpath_attr(pos: vec2<f32>, start_idx: u32) -> SubpathAttr {
    let points_len = arrayLength(&points);

    var attr: SubpathAttr;
    attr.end_idx = points_len;
    attr.nearest_idx = 0u;
    attr.d = 3.40282346638528859812e38;
    attr.sgn = 1.0;
    attr.debug = vec4(1.0, 1.0, 1.0, 1.0);

    let n = points_len;
    for (var i = start_idx; i < n; i++) {
        let p1 = point(i);
        let h1 = next_handle(i);
        let h2 = prev_handle(i + 1u);
        let p2 = point(i + 1u);
        if length(h1 - p1) == 0.0 {
            attr.end_idx = i;
            // attr.debug = vec4(0.0, 0.5, 0.5, 1.0);
            break;
        }

        let dist = sqrt(distance_bezier_sq(pos, p1, h1, h2, p2));
        if dist < attr.d {
            attr.d = dist;
            attr.nearest_idx = i;
        }
        // if is_closed(i) {
            let res = sign_bezier(pos, p1, h1, h2, p2);
            attr.sgn *= res.sgn;
            attr.debug = res.debug_color;
        // }
    }

    return attr;
}

fn render(pos: vec2<f32>) -> vec4<f32> {
    let points_len = arrayLength(&points);

    var idx = 0u;
    var d = 3.40282346638528859812e38;
    var sgn = 1.0;
    var debug_color = vec4(0.0, 0.0, 0.0, 1.0);

    var start_idx = 0u;
    while start_idx < points_len {
        let attr = get_subpath_attr(pos, start_idx);
        if attr.d < d {
            idx = attr.nearest_idx;
            d = attr.d;
            debug_color = attr.debug;
        }
        sgn *= attr.sgn;
        start_idx = attr.end_idx + 1;
    }

    let sgn_d = sgn * d;
    // return vec4(1.0);
    // return vec4(vec3(d), 1.0);
    // return vec4(vec3(sgn_d), 1.0);

    let e = point(idx + 1u).xy - point(idx).xy;
    let w = pos.xy - point(idx).xy;
    let ratio = clamp(dot(w, e) / dot(e, e), 0.0, 1.0);

    // TODO: Antialias
    var fill_rgba: vec4<f32> = select(vec4(0.0), mix(fill_rgbas[idx], fill_rgbas[idx + 1], ratio), is_closed(idx));
    fill_rgba.a *= smoothstep(1.0, - 1.0, (sgn_d));

    let stroke_width = mix(stroke_widths[idx], stroke_widths[idx + 1], ratio);
    // var stroke_rgba: vec4<f32> = mix(stroke_rgbas[idx], stroke_rgbas[idx + 1], ratio);
    var stroke_rgba: vec4<f32> = debug_color;
    stroke_rgba.a *= smoothstep(1.0, - 1.0, (d - stroke_width));

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
    var f_color: vec4<f32> = vec4(1.0, 0.0, 0.0, 1.0);
    f_color = render(pos);
    // let attr = get_subpath_attr(pos, 14u);
    // f_color = vec4(f32(attr.d));
    // f_color = blend_color(f_color, render_control_points(pos));
    return f_color;
}

struct VertexOutput {
    @builtin(position) frag_pos: vec4<f32>,
    @location(0) pos: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    let x = vertex_index & 1u;
    let min_x = f32(clip_box.min_x);
    let max_x = f32(clip_box.max_x);
    let min_y = f32(clip_box.min_y);
    let max_y = f32(clip_box.max_y);
    let max_w = f32(clip_box.max_w);

    // let x_base = min_x - max_w;
    // let x_offset = (max_x - min_x + max_w * 2.0) * f32((vertex_index >> 1u) & 1u);

    // let y_base = min_y - max_w;
    // let y_offset = (max_y - min_y + max_w * 2.0) * f32(vertex_index & 1u);

    // let clip_point = vec2(
    //     x_base + x_offset,
    //     y_base + y_offset
    // );
    var clip_point: vec2<f32>;
    clip_point.x = select(max_x + max_w, min_x - max_w, (vertex_index & 2u) == 0u);
    clip_point.y = select(max_y + max_w, min_y - max_w, (vertex_index & 1u) == 0u);

    out.frag_pos = vec4(clip_point / cam_uniforms.half_frame_size, 0.0, 1.0);
    out.pos = clip_point;
    return out;
}