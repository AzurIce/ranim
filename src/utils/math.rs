use glam::{vec2, IVec2, Vec2};
use vello::kurbo;

/// A rectangle in 2D space
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    min: Vec2,
    max: Vec2,
}

impl From<kurbo::Rect> for Rect {
    fn from(rect: kurbo::Rect) -> Self {
        Self {
            min: vec2(rect.x0 as f32, rect.y0 as f32),
            max: vec2(rect.x1 as f32, rect.y1 as f32),
        }
    }
}

impl Rect {
    pub fn union(&self, other: &Self) -> Self {
        let min = self.min.min(other.min);
        let max = self.max.max(other.max);
        Self { min, max }
    }
    pub fn intersection(&self, other: &Self) -> Self {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        Self { min, max }
    }

    pub fn center(&self) -> Vec2 {
        (self.min + self.max) / 2.0
    }


    /// Get the point of the rectangle
    /// ```text
    /// (-1,-1)-----(0,-1)-----(1,-1)
    ///    |          |          |
    /// (-1, 0)-----(0, 0)-----(1, 0)
    ///    |          |          |
    /// (-1, 1)-----(0, 1)-----(1, 1)
    /// ```
    pub fn point(&self, edge: IVec2) -> Vec2 {
        let min = self.min;
        let center = self.center();
        let max = self.max;

        let x = if edge.x < 0 { min.x } else if edge.x == 0 { center.x } else { max.x };
        let y = if edge.y < 0 { min.y } else if edge.y == 0 { center.y } else { max.y };

        vec2(x, y)
    }
}