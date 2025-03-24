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
    n: u32,
    root: array<f32, 3>,
}

fn solve_cubic(a: f32, b: f32, c: f32) -> SolveCubicRes {
    let p = b - a * a / 3.0;
    // let sqrt_neg_p = sqrt(-p);
    // let sqrt_neg_p3 = sqrt_neg_p * sqrt_neg_p * sqrt_neg_p;
    let p3 = p * p * p;

    let q = a * (2.0 * a * a - 9.0 * b) / 27.0 + c;
    let d = q * q + 4.0 * p3 / 27.0;
    let offset = - a / 3.0;

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
        res.n = 1u;
        res.root[0] = r;
        return res;
    }
    let u = sqrt(- p / 3.0);
    let v = acos(- sqrt(- 27.0 / p3) * q / 2.0) / 3.0;
    let m = cos(v);
    let n = sin(v) * 1.732050808;

    var r = vec3(m + m, - n - m, n - m) * u + offset;

    // let f = ((r + a) * r + b) * r + c;
    // let f_prime = (3.0 * r + 2.0 * a) * r + b;

    // r -= f / f_prime;

    res.n = 3u;
    res.root[0] = r.x;
    res.root[1] = r.y;
    res.root[2] = r.z;
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

fn tan_bezier(t: f32, p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p3: vec2<f32>) -> vec2<f32> {
    // The tangent of a cubic Bezier curve is the derivative of the curve equation
    // B'(t) = 3(1-t)²(p1-p0) + 6(1-t)t(p2-p1) + 3t²(p3-p2)
    let t1 = 1.0 - t;
    let term1 = 3.0 * t1 * t1 * (p1 - p0);
    let term2 = 6.0 * t1 * t * (p2 - p1);
    let term3 = 3.0 * t * t * (p3 - p2);
    return term1 + term2 + term3;
}

fn sign_bezier(pos: vec2<f32>, p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p3: vec2<f32>) -> f32 {
    // Coeffecient of the equation `cubic bezier.y = pos.y`
    let cu = (- p0.y + 3.0 * p1.y - 3.0 * p2.y + p3.y);
    let qu = (3.0 * p0.y - 6.0 * p1.y + 3.0 * p2.y);
    let li = (- 3.0 * p0.y + 3.0 * p1.y);
    let co = p0.y - pos.y;

    // if distance(p0, vec2(450.0053, 540.0)) < 1e-6 {
    //     return -1.0;
    // }
    // if abs(p0.x - 450.0053) < 1e-6 {
    //     return -1.0;
    // }
    // if abs(p0.y - 540.0) < 1e-6 {
    //     return -1.0;
    // }
    // if distance(p1, vec2(-90.075745, 540.0)) < 1e-6 {
    //     return -1.0;
    // }
    // if abs(p1.x - -90.075745) < 1e-6 {
    //     return -1.0;
    // }
    // if abs(p1.y - 540.0) < 1e-6 {
    //     return -1.0;
    // }
    // if distance(p2, vec2(-450.00528, 179.99673)) < 1e-6 {
    //     return -1.0;
    // }
    // if abs(p2.x - -450.00528) < 1e-6 {
    //     return -1.0;
    // }
    // if abs(p2.y - 179.99673) < 1e-6 {
    //     return -1.0;
    // }
    // if distance(p3, vec2(-450.00528, -539.99994)) < 1e-6 {
    //     return -1.0;
    // }
    // if abs(p3.x - -450.00528) < 1e-6 {
    //     return -1.0;
    // }
    // if abs(p3.y - -539.99994) < 1e-6 {
    //     return -1.0;
    // }
    
    

    //! ? only 1e-4 is ok
    // if abs(cu - 0.00987) < 1e-4 {
    //     return -1.0;
    // }
    // if abs(qu + 1080.00981) < 1e-6 {
    //     return -1.0;
    // }
    // if abs(li) < 1e-6 {
    //     return -1.0;
    // }


    var sgn = 1.0;

    // if abs(cu - 0.0) < 1e-6 {
    //     sgn = -1.0;
    // } else {
        let res = solve_cubic(qu / cu, li / cu, co / cu);
        // if res.n == 3u {
        //     return -1.0;
        // }
        // if 0.0 <= res.root[1] && res.root[1] <= 1.0 {
        //     return -1.0;
        // }
        let root = res.root[2];
        let root2 = root * root;
        let root3 = root2 * root;
        let A = - p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x;
        let B = 3.0 * p0.x - 6.0 * p1.x + 3.0 * p2.x;
        let C = - 3.0 * p0.x + 3.0 * p1.x;
        let D = p0.x;
        //! ? only 1e-4 is ok
        // if abs(A - 179.778025) < 1e-4 {
        //     return -1.0;
        // }
        //! ? only 1e-4 is ok
        // if abs(B - 540.45453) < 1e-4 {
        //     return -1.0;
        // }
        // if abs(C + 1620.243135) < 1e-6 {
        //     return -1.0;
        // }
        // if abs(D - 450.0053) < 1e-6 {
        //     return -1.0;
        // }
        
        // condition is true ???
        // if abs(root - 0.0) < 1e-6 && res.n == 3u {
        //     return -1.0;
        // }

        // 寄
        // let x_pos = A * root3 + B * root2 + C * root + D;
        // if x_pos < pos.x {
        //     return -1.0;
        // }
        // if abs(x_pos - 450.0053) < 1e-6 {
        //     return -1.0;
        // }
        // if 0.0 <= res.root[2] && res.root[2] <= 1.0 {
        //     return -1.0;
        // }

        // return res.root[0];
        // for (var i = 0u; i < res.n; i++) {
        //     let root = res.root[i];
        //     sgn *= sign_root(root, pos, p0, p1, p2, p3);
        // }
    // }

    // quadratic
    // For example, when:
    //
    //  + handle1    + handle2
    //
    //  + anchor1    + anchor2
    // The equation degenerate to a quadratic equation
    // if abs(cu - 0.0) < 1e-6 {
    //     sgn = -1.0;
    //     let d = li * li - 4.0 * qu * co;
    //     if d > 0.0 {
    //         let root1 = (- li - sqrt(d)) / (2.0 * qu);
    //         sgn *= sign_root(root1, pos, p0, p1, p2, p3);
    //         let root2 = (- li + sqrt(d)) / (2.0 * qu);
    //         sgn *= sign_root(root2, pos, p0, p1, p2, p3);
    //     }
    //     else if d == 0.0 {
    //         let root = - li / (2.0 * qu);
    //         // This is a workaround to fix the case where the tangent on the root is horizontal
    //         if tan_bezier(root, p0, p1, p2, p3).y != 0.0 {
    //             sgn *= sign_root(root, pos, p0, p1, p2, p3);
    //         }
    //     }
    // }
    // else {
    //     let root = solve_cubic(qu / cu, li / cu, co / cu);
    //     // sgn *= sign_root(root.x, pos, p0, p1, p2, p3);
    //     // sgn *= sign_root(root.y, pos, p0, p1, p2, p3);
    //     // sgn *= sign_root(root.z, pos, p0, p1, p2, p3);

    //     // if root.x > 100000 {
    //     //     sgn = -1.0;
    //     // }
    //     // if root.z >= 0.0 && root.z <= 1.0 {
    //         sgn = root.z;
    //     // }
    //     // if abs(root.y + root.z) < 0.01 {
    //     //     sgn = -1.0;
    //     // }
    //     // if root.z != root.z {
    //     //     sgn = -1.0;
    //     // }
    //     // if root.z == root.z {
    //     //     sgn = -1.0;
    //     // }

    //     // sgn=-1.0;
    // }

    // let tan1 = p0 - p1;
    // let tan2 = p2 - p3;
    // let nor1 = vec2(tan1.y, - tan1.x);
    // let nor2 = vec2(tan2.y, - tan2.x);

    // let cond1: vec3<bool> = vec3(p0.y < p1.y, pos.y <= p0.y, dot(pos - p0.xy, nor1) < 0.0);
    // sgn *= select(1.0, - 1.0, all(cond1) || !any(cond1));

    // let cond2: vec3<bool> = vec3(p2.y < p3.y, pos.y> p3.y, dot(pos - p3.xy, nor2) < 0.0);
    // sgn *= select(1.0, - 1.0, all(cond2) || !any(cond2));
    return sgn;
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
    // ! i < 1 just for test, the input only has 1 subpath
    for (var i = start_idx; i < 1; i++) {
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
        attr.sgn = sign_bezier(pos, p1, h1, h2, p2);
        // }
    }

    return attr;
}

fn render(pos: vec2<f32>) -> vec4<f32> {
    let points_len = arrayLength(&points);

    var idx = 0u;
    var d = 3.40282346638528859812e38;
    var sgn = 1.0;

    // var debug: vec4<f32> = vec4(1.0, 1.0, 1.0, 1.0);
    var start_idx = 0u;
    while start_idx < points_len {
        let attr = get_subpath_attr(pos, start_idx);
        if attr.d < d {
            idx = attr.nearest_idx;
            d = attr.d;
        }
        sgn *= attr.sgn;
        start_idx = attr.end_idx + 1;
        // debug = attr.debug;
    }

    // return vec4(vec3(sign_bezier(pos, point(idx), next_handle(idx), prev_handle(idx + 1u), point(idx + 1u))), 1.0);
    let sgn_d = sgn * d;
    // return vec4(1.0);
    // return vec4(vec3(d), 1.0);
    // return vec4(vec3(sgn), 1.0);
    return vec4(vec3(sgn_d), 1.0);
    // return debug;
    // return vec4(1.0);

    // let e = point(idx + 1u).xy - point(idx).xy;
    // let w = pos.xy - point(idx).xy;
    // let ratio = clamp(dot(w, e) / dot(e, e), 0.0, 1.0);
    // let anchor_index = idx / 2;

    // // TODO: Antialias
    // var fill_rgba: vec4<f32> = select(vec4(0.0), mix(fill_rgbas[anchor_index], fill_rgbas[anchor_index + 1], ratio), is_closed(idx));
    // fill_rgba.a *= smoothstep(1.0, - 1.0, (sgn_d));

    // let stroke_width = mix(stroke_widths[anchor_index], stroke_widths[anchor_index + 1], ratio);
    // var stroke_rgba: vec4<f32> = mix(stroke_rgbas[anchor_index], stroke_rgbas[anchor_index + 1], ratio);
    // stroke_rgba.a *= smoothstep(1.0, - 1.0, (d - stroke_width));

    // var f_color = blend_color(stroke_rgba, fill_rgba);

    // return f_color;
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