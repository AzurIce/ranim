use glam::Vec3;
use log::warn;

use crate::rabject::{rabject2d::RabjectEntity2d, Blueprint};

use super::{VPath, VPathPoint};

/// A blueprint for [`VPath`]
///
/// It's underlying data structure is a list of [`VPathPoint`],
/// Represents the anchors and their information of a cubic bezier path.
pub struct VPathBuilder {
    points: Vec<VPathPoint>,
}

impl Blueprint<RabjectEntity2d<VPath>> for VPathBuilder {
    fn build(self) -> RabjectEntity2d<VPath> {
        VPath {
            points: self.points,
            ..Default::default()
        }
        .into()
    }
}

impl VPathBuilder {
    /// Create a new [`VPathBuilder`] with a starting point
    pub fn start(pos: Vec3) -> Self {
        Self {
            points: vec![VPathPoint::new(pos, None, None)],
        }
    }

    /// Add a line to the path
    ///
    /// A line can be present by a cubic bezier curve with
    /// p0.next_handle and p1.prev_handle set with (p0 + p1) / 2
    pub fn line_to(mut self, pos: Vec3) -> Self {
        assert!(!self.points.is_empty());

        let mid = (self.points.last().unwrap().position + pos) / 2.0;
        self.points.last_mut().unwrap().next_handle = Some(mid);
        self.points.push(VPathPoint::new(pos, Some(mid), None));
        self
    }

    /// Add a quadratic bezier curve to the path
    ///
    /// A quadratic bezier curve can be present by a cubic bezier curve with
    /// p0.next_handle = p1.prev_handle
    pub fn quad_to(mut self, pos: Vec3, h: Vec3) -> Self {
        assert!(!self.points.is_empty());

        self.points.last_mut().unwrap().next_handle = Some(h);
        self.points.push(VPathPoint::new(pos, Some(h), None));
        self
    }

    /// Add a cubic bezier curve to the path
    pub fn cubic_to(mut self, pos: Vec3, h0: Vec3, h1: Vec3) -> Self {
        assert!(!self.points.is_empty());

        self.points.last_mut().unwrap().next_handle = Some(h0);
        self.points.push(VPathPoint::new(pos, Some(h1), None));
        self
    }

    /// Close the path by connecting the first and last points
    ///
    /// The prev_handle of the first point will be set with the last point's next_handle,
    /// and the next_handle of the last point will be set with the first point's prev_handle.
    ///
    /// Should make sure that the last point's position is equal to the first point's position.
    pub fn close(mut self) -> Self {
        assert!(self.points.len() >= 2); // or 3?
        if self.points.first().unwrap().position != self.points.last().unwrap().position {
            warn!("The first and last points of the path are not the same");
            let start = self.points[0].position;
            self = self.line_to(start);
        }
        assert_eq!(
            self.points.first().unwrap().position,
            self.points.last().unwrap().position
        );
        self.points.first_mut().unwrap().prev_handle = self.points.last().unwrap().prev_handle;
        self.points.first_mut().unwrap().prev_handle = self.points.last().unwrap().prev_handle;
        self.points.last_mut().unwrap().next_handle = self.points.first().unwrap().next_handle;
        self
    }
}
