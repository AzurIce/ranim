
struct MobjectVertexData {
    point: [f32; 3],
    colors: [f32; 4],
}

pub struct Mobject {
    data: Vec<MobjectVertexData>,
}

struct VMobjectVertexData {
    point: [f32; 3],
    stroke_rgba: [f32; 4],
    stroke_width: f32,
    joint_angle: f32,
    fill_rgba: [f32; 4],
    base_normal: [f32; 3],
    fill_border_width: f32,
}

pub struct VMobject {
    data: Vec<VMobjectVertexData>,
}