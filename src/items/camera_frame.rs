// MARK: CameraFrame

use glam::{Mat4, Vec3, vec2};

use crate::prelude::{Alignable, Interpolatable};

/// The data of a camera
///
/// The [`CameraFrame`] has a [`CameraFrame::perspective_blend`] property (default is `0.0`),
/// which is used to blend between orthographic and perspective projection.
///
/// See [`CameraFrame::DEFAULT`] for the default value.
#[derive(Clone, Debug)]
pub struct CameraFrame {
    pub pos: Vec3,
    pub up: Vec3,
    pub facing: Vec3,
    /// Used in orthographic projection
    pub scale: f32,
    /// Used in perspective projection
    pub fovy: f32,
    // far > near
    pub near: f32,
    pub far: f32,
    pub perspective_blend: f32,
}

impl Interpolatable for CameraFrame {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Self {
            pos: self.pos.lerp(target.pos, t),
            up: self.up.lerp(target.up, t),
            facing: self.facing.lerp(target.facing, t),
            scale: self.scale.lerp(&target.scale, t),
            fovy: self.fovy.lerp(&target.fovy, t),
            near: self.near.lerp(&target.near, t),
            far: self.far.lerp(&target.far, t),
            perspective_blend: self
                .perspective_blend
                .lerp(&target.perspective_blend, t)
                .clamp(0.0, 1.0),
        }
    }
}

impl Alignable for CameraFrame {
    fn is_aligned(&self, _other: &Self) -> bool {
        true
    }
    fn align_with(&mut self, _other: &mut Self) {}
}

impl Default for CameraFrame {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl CameraFrame {
    /// The default value of [`CameraFrame`]
    pub const DEFAULT: Self = Self {
        pos: Vec3::ZERO,
        up: Vec3::Y,
        facing: Vec3::NEG_Z,

        scale: 1.0,
        fovy: std::f32::consts::PI / 2.0,

        near: -1000.0,
        far: 1000.0,
        perspective_blend: 0.0,
    };
    /// Create a new CameraFrame at the origin facing to the negative z-axis and use Y as up vector.
    ///
    /// See [`CameraFrame::DEFAULT`] for the default projection settings.
    pub fn new() -> Self {
        Self::default()
    }
}

impl CameraFrame {
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.pos, self.pos + self.facing, self.up)
    }

    /// Use the given frame size as `left`, `right`, `bottom`, `top` to construct an orthographic matrix
    pub fn orthographic_mat(&self, frame_height: f32, aspect_ratio: f32) -> Mat4 {
        let frame_size = vec2(frame_height * aspect_ratio, frame_height);
        let frame_size = frame_size * self.scale;
        Mat4::orthographic_rh(
            -frame_size.x / 2.0,
            frame_size.x / 2.0,
            -frame_size.y / 2.0,
            frame_size.y / 2.0,
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
    pub fn projection_matrix(&self, frame_height: f32, aspect_ratio: f32) -> Mat4 {
        self.orthographic_mat(frame_height, aspect_ratio)
            .lerp(&self.perspective_mat(aspect_ratio), self.perspective_blend)
    }

    /// Use the given frame size to construct view projection matrix
    pub fn view_projection_matrix(&self, frame_height: f32, aspect_ratio: f32) -> Mat4 {
        self.projection_matrix(frame_height, aspect_ratio) * self.view_matrix()
    }
}

impl CameraFrame {
    /// Center the canvas in the frame when [`CameraFrame::perspective_blend`] is `1.0`
    pub fn center_canvas_in_frame(
        &mut self,
        center: Vec3,
        width: f32,
        height: f32,
        up: Vec3,
        normal: Vec3,
        aspect_ratio: f32,
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
