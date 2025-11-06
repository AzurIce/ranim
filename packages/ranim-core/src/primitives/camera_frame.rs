// MARK: CameraFrame

use glam::{DMat4, DVec3, dvec2};

use crate::{
    Extract,
    prelude::{Alignable, Interpolatable},
    primitives::CoreItem,
};

/// The data of a camera
///
/// The [`CameraFrame`] has a [`CameraFrame::perspective_blend`] property (default is `0.0`),
/// which is used to blend between orthographic and perspective projection.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraFrame {
    /// The position
    pub pos: DVec3,
    /// The up unit vec
    pub up: DVec3,
    /// The facing unit vec
    pub facing: DVec3,
    /// The scaling factor, used in orthographic projection
    pub scale: f64,
    /// The field of view angle, used in perspective projection
    pub fovy: f64,
    // far > near
    /// The near pane
    pub near: f64,
    /// The far pane
    pub far: f64,
    /// The perspective blend value in [0.0, 1.0]
    pub perspective_blend: f64,
}

impl Extract for CameraFrame {
    type Target = CoreItem;
    fn extract(&self) -> Vec<Self::Target> {
        vec![CoreItem::CameraFrame(self.clone())]
    }
}
// impl Primitive for CameraFrame {
//     fn build_primitives<T: IntoIterator<Item = Self>>(iter: T) -> super::Primitives {
//         Primitives::CameraFrame(iter.into_iter().collect())
//     }
// }

impl Interpolatable for CameraFrame {
    fn lerp(&self, target: &Self, t: f64) -> Self {
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
        Self {
            pos: DVec3::ZERO,
            up: DVec3::Y,
            facing: DVec3::NEG_Z,

            scale: 1.0,
            fovy: std::f64::consts::PI / 2.0,

            near: -1000.0,
            far: 1000.0,
            perspective_blend: 0.0,
        }
    }
}

impl CameraFrame {
    /// Create a new CameraFrame at the origin facing to the negative z-axis and use Y as up vector with default projection settings.
    pub fn new() -> Self {
        Self::default()
    }
}

impl CameraFrame {
    /// The view matrix of the camera
    pub fn view_matrix(&self) -> DMat4 {
        DMat4::look_at_rh(self.pos, self.pos + self.facing, self.up)
    }

    /// Use the given frame size as `left`, `right`, `bottom`, `top` to construct an orthographic matrix
    pub fn orthographic_mat(&self, frame_height: f64, aspect_ratio: f64) -> DMat4 {
        let frame_size = dvec2(frame_height * aspect_ratio, frame_height);
        let frame_size = frame_size * self.scale;
        DMat4::orthographic_rh(
            -frame_size.x / 2.0,
            frame_size.x / 2.0,
            -frame_size.y / 2.0,
            frame_size.y / 2.0,
            self.near,
            self.far,
        )
    }

    /// Use the given frame aspect ratio to construct a perspective matrix
    pub fn perspective_mat(&self, aspect_ratio: f64) -> DMat4 {
        let near = self.near.max(0.1);
        let far = self.far.max(near);
        DMat4::perspective_rh(self.fovy, aspect_ratio, near, far)
    }

    /// Use the given frame size to construct projection matrix
    pub fn projection_matrix(&self, frame_height: f64, aspect_ratio: f64) -> DMat4 {
        self.orthographic_mat(frame_height, aspect_ratio)
            .lerp(&self.perspective_mat(aspect_ratio), self.perspective_blend)
    }

    /// Use the given frame size to construct view projection matrix
    pub fn view_projection_matrix(&self, frame_height: f64, aspect_ratio: f64) -> DMat4 {
        self.projection_matrix(frame_height, aspect_ratio) * self.view_matrix()
    }
}

impl CameraFrame {
    /// Center the canvas in the frame when [`CameraFrame::perspective_blend`] is `1.0`
    pub fn center_canvas_in_frame(
        &mut self,
        center: DVec3,
        width: f64,
        height: f64,
        up: DVec3,
        normal: DVec3,
        aspect_ratio: f64,
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
