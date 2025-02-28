struct Point {
    pos: vec3<f32>,
    stroke_width: f32,
    stroke_color: vec4<f32>,
    fill_color: vec4<f32>,
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
@group(0) @binding(1) var<storage> joint_angles: array<f32>;
@group(0) @binding(2) var<storage, read_write> vertices: array<Vertex>;
@group(0) @binding(3) var<uniform> uniforms: ComputeUniform;

fn point_on_quadratic(t: f32, c0: vec3<f32>, c1: vec3<f32>, c2: vec3<f32>) -> vec3<f32> {
    return c0 + t * (c1 + t * c2);
}

fn tangent_on_quadratic(t: f32, c1: vec3<f32>, c2: vec3<f32>) -> vec3<f32> {
    return c1 + 2.0 * t * c2;
}

const MAX_STEP = 16u;

@compute
@workgroup_size(1)
fn cs_main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let p0 = points[global_invocation_id.x * 2];
    let p1 = points[global_invocation_id.x * 2 + 1];
    let p2 = points[global_invocation_id.x * 2 + 2];

    let c0 = p0.pos;
    let c1 = 2.0 * (p1.pos - p0.pos);
    let c2 = p0.pos - 2.0 * p1.pos + p2.pos;

    // 8 * 2 = 16 vertices per invocation
    for (var i = 0u; i < MAX_STEP; i ++) {
        let t = f32(i) / f32(MAX_STEP - 1);
        var vertex: Vertex;
        // vertex.pos = vec4<f32>(point_on_quadratic(t, c0, c1, c2), 1.0);
        // vertex.pos = vec4<f32>(p0.pos, 1.0);
        // vertex.color = vec4<f32>(1.0, 1.0, 1.0, 1.0);

        // vertices[global_invocation_id.x * 8 + i] = vertex;
        // let point = mix(p0.pos, p2.pos, t);
        let point = point_on_quadratic(t, c0, c1, c2);
        let tangent = tangent_on_quadratic(t, c1, c2);

        var step = normalize(cross(uniforms.unit_normal, tangent));
        let side = t == 0.0 || t == 1.0;
        if side {
            var angle = 0.0;
            if i == 0 {
                angle = joint_angles[global_invocation_id.x];
            } else {
                angle = -joint_angles[global_invocation_id.x + 1];
            }

            let cos_angle = cos(angle);
            let sin_angle = sin(angle);

            if abs(cos_angle) < 0.99 {
                let shift = (-cos_angle + 1) / sin_angle;
                step += shift * normalize(tangent);
            }
        }

        let width = mix(p0.stroke_width, p2.stroke_width, t);
        let color = mix(p0.stroke_color, p2.stroke_color, t);

        for (var sign = -1; sign <= 1; sign += 2) {
            let dist_to_curve = f32(sign) * 0.5 * width;

            var vertex: Vertex;
            vertex.pos = vec4<f32>(point + dist_to_curve * step, 1.0);
            vertex.color = color;

            vertices[global_invocation_id.x * MAX_STEP * 2 + i * 2 + (u32(sign) + 1) / 2] = vertex;
        }
    }
}

