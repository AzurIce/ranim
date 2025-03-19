// MARK: CameraFrame

use glam::{Mat4, Vec2, Vec3, vec2};

use crate::prelude::{Alignable, Interpolatable};

/// Default pos is at the origin, looking to the negative z-axis
#[derive(Clone, Debug)]
pub struct CameraFrame {
    pub fovy: f32,
    // pub size: (usize, usize),
    pub pos: Vec3,
    pub up: Vec3,
    pub facing: Vec3,
    // pub rotation: Mat4,
    // far > near
    pub near: f32,
    pub far: f32,
    pub perspective_blend: f32,
}

impl Interpolatable for CameraFrame {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Self {
            fovy: self.fovy.lerp(&target.fovy, t),
            pos: self.pos.lerp(target.pos, t),
            up: self.up.lerp(target.up, t),
            facing: self.facing.lerp(target.facing, t),
            near: self.near.lerp(&target.near, t),
            far: self.far.lerp(&target.far, t),
            perspective_blend: self.perspective_blend.lerp(&target.perspective_blend, t).clamp(0.0, 1.0),
        }
    }
}

impl Alignable for CameraFrame {
    fn is_aligned(&self, _other: &Self) -> bool {
        true
    }
    fn align_with(&mut self, _other: &mut Self) {}
}

impl CameraFrame {
    pub fn new_with_size(width: usize, height: usize) -> Self {
        let mut camera_frame = Self {
            fovy: std::f32::consts::PI / 2.0,
            pos: Vec3::ZERO,
            up: Vec3::Y,
            facing: Vec3::NEG_Z,
            // rotation: Mat4::IDENTITY,
            near: -1000.0,
            far: 1000.0,
            perspective_blend: 0.0,
        };
        camera_frame.center_canvas_in_frame(
            Vec3::ZERO,
            width as f32,
            height as f32,
            Vec3::Y,
            Vec3::Z,
            width as f32 / height as f32,
        );
        camera_frame
    }
}

impl CameraFrame {
    pub fn view_matrix(&self) -> Mat4 {
        // Mat4::look_at_rh(vec3(0.0, 0.0, 1080.0), Vec3::NEG_Z, Vec3::Y)
        Mat4::look_at_rh(self.pos, self.pos + self.facing, self.up)
    }

    /// Use the given frame size as `left`, `right`, `bottom`, `top` to construct an orthographic matrix
    pub fn orthographic_mat(&self, frame_width: f32, frame_height: f32) -> Mat4 {
        Mat4::orthographic_rh(
            -frame_width / 2.0,
            frame_width / 2.0,
            -frame_height / 2.0,
            frame_height / 2.0,
            self.near,
            self.far,
        )
    }

    /// Use the given frame aspect ratio to construct a perspective matrix
    pub fn perspective_mat(&self, aspect_ratio: f32) -> Mat4 {
        let near = self.near.max(0.1);
        let far = self.far.max(near);
        Mat4::perspective_rh(self.fovy, aspect_ratio, near, far)
    }

    /// Use the given frame size to construct projection matrix
    pub fn projection_matrix(&self, frame_width: f32, frame_height: f32) -> Mat4 {
        let aspect_ratio = frame_width / frame_height;
        self.orthographic_mat(frame_width, frame_height)
            .lerp(&self.perspective_mat(aspect_ratio), self.perspective_blend)
    }

    pub fn view_projection_matrix(&self, frame_width: f32, frame_height: f32) -> Mat4 {
        self.projection_matrix(frame_width, frame_height) * self.view_matrix()
    }
}

impl CameraFrame {
    pub fn set_fovy(&mut self, fovy: f32) -> &mut Self {
        self.fovy = fovy;
        self
    }

    pub fn move_to(&mut self, pos: Vec3) -> &mut Self {
        self.pos = pos;
        self
    }

    pub fn center_canvas_in_frame(
        &mut self,
        center: Vec3,
        width: f32,
        height: f32,
        up: Vec3,
        normal: Vec3,
        aspect_ratio: f32
    ) -> &mut Self {
        let canvas_ratio = height / width;
        let up = up.normalize();
        let normal = normal.normalize();

        let height = if aspect_ratio > canvas_ratio {
            height
        } else {
            width / aspect_ratio
        };

        let distance = height * 0.5 / (0.5 * self.fovy).tan();

        self.up = up;
        self.pos = center + normal * distance;
        self.facing = -normal;
        self
    }
}
