// Per-item metadata
struct ItemInfo {
    point_offset: u32,
    point_count: u32,
    attr_offset: u32,
    attr_count: u32,
}

struct ClipBox {
    min_x: atomic<i32>,
    max_x: atomic<i32>,
    min_y: atomic<i32>,
    max_y: atomic<i32>,
    max_w: atomic<i32>,
}

struct Basis {
    u: vec3<f32>,
    v: vec3<f32>,
}

struct PlaneData {
    normal: vec4<f32>,
    origin: vec4<f32>,
}

fn basis_from_normal(n: vec3<f32>) -> Basis {
    let arbitrary = select(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), abs(n.x) > 0.99);
    let basis_u = normalize(cross(n, arbitrary));
    let basis_v = cross(n, basis_u);
    return Basis(basis_u, basis_v);
}

@group(0) @binding(0) var<storage> item_infos: array<ItemInfo>;
@group(0) @binding(1) var<storage> planes: array<PlaneData>;
@group(0) @binding(2) var<storage> points3d: array<vec4<f32>>;
@group(0) @binding(3) var<storage> stroke_widths: array<f32>;
@group(0) @binding(4) var<storage, read_write> points2d: array<vec4<f32>>;
// clip_boxes: 5 i32 per item, laid out as [min_x, max_x, min_y, max_y, max_w, ...]
@group(0) @binding(5) var<storage, read_write> clip_boxes: array<atomic<i32>>;

@compute
@workgroup_size(256)
fn cs_main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
) {
    let total_points = arrayLength(&points3d);
    let index = global_invocation_id.x;
    if index >= total_points {
        return;
    }

    // Binary search to find which item this point belongs to
    let item_count = arrayLength(&item_infos);
    var lo = 0u;
    var hi = item_count;
    while lo < hi {
        let mid = (lo + hi) / 2u;
        let info = item_infos[mid];
        if index < info.point_offset {
            hi = mid;
        } else if index >= info.point_offset + info.point_count {
            lo = mid + 1u;
        } else {
            lo = mid;
            break;
        }
    }
    let item_idx = lo;
    let info = item_infos[item_idx];
    let plane = planes[item_idx];
    let basis = basis_from_normal(plane.normal.xyz);
    let plane_origin = plane.origin.xyz;

    let p_vec = points3d[index];
    let p = p_vec.xyz;
    let is_closed = p_vec.w;
    let diff = p - plane_origin;

    let x = dot(diff, basis.u);
    let y = dot(diff, basis.v);

    // Local index within this item's points
    let local_idx = index - info.point_offset;
    let w = stroke_widths[info.attr_offset + local_idx / 2u];

    points2d[index] = vec4(x, y, is_closed, 0.0);

    let scale = 1000.0;
    let clip_base = item_idx * 5u;
    atomicMin(&clip_boxes[clip_base + 0u], i32(floor(x * scale)));
    atomicMax(&clip_boxes[clip_base + 1u], i32(ceil(x * scale)));
    atomicMin(&clip_boxes[clip_base + 2u], i32(floor(y * scale)));
    atomicMax(&clip_boxes[clip_base + 3u], i32(ceil(y * scale)));
    atomicMax(&clip_boxes[clip_base + 4u], i32(ceil(w * scale)));
}
