// MARK: CameraFrame

use glam::{DMat4, DVec3, dvec2};

use crate::{
    Extract,
    animation::{AnimationCell, Eval},
    core_item::CoreItem,
    prelude::{Alignable, Interpolatable},
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

    // far > near
    /// The near pane
    pub near: f64,
    /// The far pane
    pub far: f64,
    /// The perspective blend value in [0.0, 1.0]
    pub perspective_blend: f64,

    /// **Ortho**: Top - Bottom
    pub frame_height: f64,
    /// **Ortho**: The scaling factor
    pub scale: f64,

    /// **Perspective**: The field of view angle, used in perspective projection
    pub fovy: f64,
}

impl Extract for CameraFrame {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        buf.push(CoreItem::CameraFrame(self.clone()));
    }
}

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
            frame_height: self.frame_height.lerp(&target.frame_height, t),
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

            near: -1000.0,
            far: 1000.0,
            perspective_blend: 0.0,

            scale: 1.0,
            frame_height: 8.0,

            fovy: std::f64::consts::PI / 2.0,
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
    /// Set the view matrix of the camera.
    pub fn set_view_matrix(&mut self, view_matrix: DMat4) {
        let inv = view_matrix.inverse();
        self.pos = inv.transform_point3(DVec3::ZERO);
        self.up = inv.transform_vector3(DVec3::Y).normalize();
        self.facing = inv.transform_vector3(DVec3::NEG_Z).normalize();
    }

    /// Set the view matrix of the camera and return the modified `Self`.
    pub fn with_view_matrix(mut self, view_matrix: DMat4) -> Self {
        self.set_view_matrix(view_matrix);
        self
    }

    /// The view matrix of the camera
    pub fn view_matrix(&self) -> DMat4 {
        DMat4::look_to_rh(self.pos, self.facing, self.up)
    }

    /// Use the given frame size as `left`, `right`, `bottom`, `top` to construct an orthographic matrix
    pub fn orthographic_mat(&self, aspect_ratio: f64) -> DMat4 {
        let frame_size = dvec2(self.frame_height * aspect_ratio, self.frame_height);
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
    pub fn projection_matrix(&self, aspect_ratio: f64) -> DMat4 {
        self.orthographic_mat(aspect_ratio)
            .lerp(&self.perspective_mat(aspect_ratio), self.perspective_blend)
    }

    /// Use the given frame size to construct view projection matrix
    pub fn view_projection_matrix(&self, aspect_ratio: f64) -> DMat4 {
        self.projection_matrix(aspect_ratio) * self.view_matrix()
    }
}

impl CameraFrame {
    /// Create a perspective camera positioned using spherical coordinates (Z-up), looking at the origin.
    ///
    /// - `phi`: polar angle from +Z axis in radians (0 = straight up along +Z, π/2 = XY plane)
    /// - `theta`: azimuth angle in radians (0 = +X direction, π/2 = +Y direction)
    /// - `distance`: distance from the origin
    pub fn from_spherical(phi: f64, theta: f64, distance: f64) -> Self {
        let mut cam = Self {
            perspective_blend: 1.0,
            up: DVec3::Z,
            ..Self::default()
        };
        cam.set_spherical(phi, theta, distance, DVec3::ZERO);
        cam
    }

    /// Position the camera using spherical coordinates (Z-up) around a target point.
    ///
    /// - `phi`: polar angle from +Z axis in radians (0 = straight up along +Z, π/2 = XY plane)
    /// - `theta`: azimuth angle in radians (0 = +X direction, π/2 = +Y direction)
    /// - `distance`: distance from `target`
    /// - `target`: the point the camera looks at
    pub fn set_spherical(
        &mut self,
        phi: f64,
        theta: f64,
        distance: f64,
        target: DVec3,
    ) -> &mut Self {
        self.pos = target
            + DVec3::new(
                distance * phi.sin() * theta.cos(),
                distance * phi.sin() * theta.sin(),
                distance * phi.cos(),
            );
        self.facing = (target - self.pos).normalize();
        self.up = DVec3::Z;
        self
    }

    /// Set the camera to look at a target point.
    pub fn look_at(&mut self, target: DVec3) -> &mut Self {
        self.facing = (target - self.pos).normalize();
        self
    }

    /// Create an orbit animation that rotates the camera around `target`
    /// by `total_angle` radians in the XY plane (Z-up).
    ///
    /// The camera's current position is used to derive the spherical
    /// coordinates (distance, elevation) which are kept constant during the orbit.
    ///
    /// # Example
    /// ```ignore
    /// use std::f64::consts::TAU;
    ///
    /// let mut cam = CameraFrame::from_spherical(phi, theta, distance);
    /// let r_cam = r.insert(cam.clone());
    /// r.timeline_mut(r_cam).play(
    ///     cam.orbit(DVec3::ZERO, TAU)
    ///        .with_duration(8.0)
    ///        .with_rate_func(linear),
    /// );
    /// ```
    pub fn orbit(&mut self, target: DVec3, total_angle: f64) -> AnimationCell<Self> {
        let offset = self.pos - target;
        let distance = offset.length();
        let phi = if distance > 0.0 {
            (offset.z / distance).acos()
        } else {
            0.0
        };
        let theta0 = offset.y.atan2(offset.x);
        let src = self.clone();

        struct Orbit {
            src: CameraFrame,
            target: DVec3,
            distance: f64,
            phi: f64,
            theta0: f64,
            total_angle: f64,
        }

        impl Eval<CameraFrame> for Orbit {
            fn eval_alpha(&self, alpha: f64) -> CameraFrame {
                let theta = self.theta0 + self.total_angle * alpha;
                let mut result = self.src.clone();
                result.set_spherical(self.phi, theta, self.distance, self.target);
                result
            }
        }

        Orbit {
            src,
            target,
            distance,
            phi,
            theta0,
            total_angle,
        }
        .into_animation_cell()
        .apply_to(self)
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

#[cfg(test)]
mod tests {
    use super::*;
    use glam::dvec3;

    #[test]
    fn test_set_view_matrix_default() {
        let camera = CameraFrame::new();
        let view_matrix = camera.view_matrix();

        let mut new_camera = CameraFrame::new();
        new_camera.set_view_matrix(view_matrix);

        assert!(new_camera.pos.distance(camera.pos) < 1e-10);
        assert!(new_camera.up.angle_between(camera.up) < 1e-10);
        assert!(new_camera.facing.angle_between(camera.facing) < 1e-10);
    }

    #[test]
    fn test_set_view_matrix_translated() {
        let mut camera = CameraFrame::new();
        camera.pos = dvec3(5.0, 3.0, -2.0);
        let view_matrix = camera.view_matrix();

        let mut new_camera = CameraFrame::new();
        new_camera.set_view_matrix(view_matrix);

        assert!(new_camera.pos.distance(camera.pos) < 1e-10);
        assert!(new_camera.up.angle_between(camera.up) < 1e-10);
        assert!(new_camera.facing.angle_between(camera.facing) < 1e-10);
    }

    #[test]
    fn test_set_view_matrix_rotated() {
        let mut camera = CameraFrame::new();
        camera.facing = dvec3(1.0, 0.0, 0.0);
        camera.up = dvec3(0.0, 1.0, 0.0);
        let view_matrix = camera.view_matrix();

        let mut new_camera = CameraFrame::new();
        new_camera.set_view_matrix(view_matrix);

        assert!(new_camera.pos.distance(camera.pos) < 1e-10);
        assert!(new_camera.up.angle_between(camera.up) < 1e-10);
        assert!(new_camera.facing.angle_between(camera.facing) < 1e-10);
    }

    #[test]
    fn test_set_view_matrix_complex() {
        let mut camera = CameraFrame::new();
        camera.pos = dvec3(10.0, 5.0, 3.0);
        camera.facing = dvec3(1.0, 0.0, 1.0).normalize();
        camera.up = dvec3(0.0, 1.0, 0.0);
        let view_matrix = camera.view_matrix();

        let mut new_camera = CameraFrame::new();
        new_camera.set_view_matrix(view_matrix);

        assert!(new_camera.pos.distance(camera.pos) < 1e-10);
        assert!(new_camera.up.angle_between(camera.up) < 1e-10);
        assert!(new_camera.facing.angle_between(camera.facing) < 1e-10);
    }
}
