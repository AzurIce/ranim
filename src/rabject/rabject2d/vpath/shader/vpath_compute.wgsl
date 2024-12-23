struct Point {
    pos: vec3<f32>,
    prev_handle: vec3<f32>,
    next_handle: vec3<f32>,
    fill_color: vec4<f32>,
    stroke_color: vec4<f32>,
    stroke_width: f32,
    joint_angle: f32,
}

struct Vertex {
    pos: vec4<f32>,
    color: vec4<f32>,
}

struct ComputeUniform {
    unit_normal: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0) var<storage> points: array<Point>;
@group(0) @binding(1) var<storage, read_write> vertices: array<Vertex>;
@group(0) @binding(2) var<uniform> uniforms: ComputeUniform;

fn point_on_cubic(t: f32, c0: vec3<f32>, c1: vec3<f32>, c2: vec3<f32>, c3: vec3<f32>) -> vec3<f32> {
    return c0 + t * (c1 + t * (c2 + t * c3));
}

fn tangent_on_cubic(t: f32, c1: vec3<f32>, c2: vec3<f32>, c3: vec3<f32>) -> vec3<f32> {
    return c1 + t * (2.0 * c2 + 3.0 * t * c3);
}

const MAX_STEP = 16u;

@compute
@workgroup_size(1)
fn cs_main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let p0 = points[global_invocation_id.x];
    let p1 = points[global_invocation_id.x + 1];

    let h0 = p0.next_handle;
    let h1 = p1.prev_handle;

    let c0 = p0.pos;
    let c1 = 3.0 * (h0 - p0.pos);
    let c2 = 3.0 * (h1 - 2.0 * h0 + p0.pos);
    let c3 = 3.0 * h0 - 3.0 * h1 + p1.pos - p0.pos;

    // 8 * 2 = 16 vertices per invocation
    for (var i = 0u; i < MAX_STEP; i ++) {
        let t = f32(i) / f32(MAX_STEP - 1);
        var vertex: Vertex;
        // vertex.pos = vec4<f32>(point_on_quadratic(t, c0, c1, c2), 1.0);
        // vertex.pos = vec4<f32>(p0.pos, 1.0);
        // vertex.color = vec4<f32>(1.0, 1.0, 1.0, 1.0);

        // vertices[global_invocation_id.x * 8 + i] = vertex;
        // let point = mix(p0.pos, p2.pos, t);
        let point = point_on_cubic(t, c0, c1, c2, c3);
        let tangent = tangent_on_cubic(t, c1, c2, c3);

        var step = normalize(cross(uniforms.unit_normal, tangent));
        let side = t == 0.0 || t == 1.0;
        if side {
            var angle = 0.0;
            if i == 0 {
                angle = p0.joint_angle;
            } else {
                angle = -p1.joint_angle;
            }

            let cos_angle = cos(angle);
            let sin_angle = sin(angle);

            if abs(cos_angle) < 0.99 {
                let shift = (-cos_angle + 1) / sin_angle;
                step += shift * normalize(tangent);
            }
        }

        let width = mix(p0.stroke_width, p1.stroke_width, t);
        let color = mix(p0.stroke_color, p1.stroke_color, t);

        for (var sign = -1; sign <= 1; sign += 2) {
            let dist_to_curve = f32(sign) * 0.5 * width;

            var vertex: Vertex;
            vertex.pos = vec4<f32>(point + dist_to_curve * step, 1.0);
            // vertex.pos = vec4<f32>(point, 1.0);
            vertex.color = color;

            vertices[global_invocation_id.x * MAX_STEP * 2 + i * 2 + (u32(sign) + 1) / 2] = vertex;
        }
    }
}

