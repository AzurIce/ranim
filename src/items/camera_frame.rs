// MARK: CameraFrame

use glam::{Mat4, Vec2, Vec3, vec2};

use crate::prelude::{Alignable, Interpolatable};

/// Default pos is at the origin, looking to the negative z-axis
#[derive(Clone, Debug)]
pub struct CameraFrame {
    pub fovy: f32,
    pub size: (usize, usize),
    pub pos: Vec3,
    pub up: Vec3,
    pub facing: Vec3,
    // pub rotation: Mat4,
}

impl Interpolatable for CameraFrame {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        assert_eq!(self.size, target.size);
        Self {
            fovy: self.fovy.lerp(&target.fovy, t),
            size: self.size,
            pos: self.pos.lerp(target.pos, t),
            up: self.up.lerp(target.up, t),
            facing: self.facing.lerp(target.facing, t),
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
            size: (width, height),
            fovy: std::f32::consts::PI / 2.0,
            pos: Vec3::ZERO,
            up: Vec3::Y,
            facing: Vec3::NEG_Z,
            // rotation: Mat4::IDENTITY,
        };
        camera_frame.center_canvas_in_frame(
            Vec3::ZERO,
            width as f32,
            height as f32,
            Vec3::Y,
            Vec3::Z,
        );
        camera_frame
    }
}

impl CameraFrame {
    pub fn ratio(&self) -> f32 {
        self.size.0 as f32 / self.size.1 as f32
    }
    pub fn view_matrix(&self) -> Mat4 {
        // Mat4::look_at_rh(vec3(0.0, 0.0, 1080.0), Vec3::NEG_Z, Vec3::Y)
        Mat4::look_at_rh(self.pos, self.pos + self.facing, self.up)
    }
    pub fn frame_size(&self) -> Vec2 {
        vec2(self.size.0 as f32, self.size.1 as f32)
    }
    pub fn half_frame_size(&self) -> Vec2 {
        self.frame_size() / 2.0
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(
            self.fovy,
            self.size.0 as f32 / self.size.1 as f32,
            0.1,
            1000.0,
        )
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
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
    ) -> &mut Self {
        let canvas_ratio = height / width;
        let up = up.normalize();
        let normal = normal.normalize();

        let height = if self.ratio() > canvas_ratio {
            height
        } else {
            width / self.ratio()
        };

        let distance = height * 0.5 / (0.5 * self.fovy).tan();

        self.up = up;
        self.pos = center + normal * distance;
        self.facing = -normal;
        // trace!(
        //     "[Camera] centered canvas in frame, pos: {:?}, facing: {:?}, up: {:?}",
        //     self.pos,
        //     self.facing,
        //     self.up
        // );

        self
    }
}
