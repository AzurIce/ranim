//! Debug utilities for visualizing item properties.

use std::ops::{Deref, DerefMut};

use ranim_core::{
    Extract,
    anchor::Aabb,
    color::{self, AlphaColor},
    core_item::CoreItem,
    glam::DVec3,
    traits::{FillColor, RotateTransform, ScaleTransform, ShiftTransform, StrokeColor},
};

use crate::vitem::{VItem, geometry::Rectangle};

/// Wrapper that visualizes the AABB of the inner item as a wireframe rectangle.
#[derive(Clone)]
pub struct VisualizeAabbItem<T: Aabb>(pub T);

impl<T: Aabb> Deref for VisualizeAabbItem<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Aabb> DerefMut for VisualizeAabbItem<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Aabb> Aabb for VisualizeAabbItem<T> {
    fn aabb(&self) -> [DVec3; 2] {
        self.0.aabb()
    }
}

impl<T: Aabb + ShiftTransform> ShiftTransform for VisualizeAabbItem<T> {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.0.shift(offset);
        self
    }
}

impl<T: Aabb + RotateTransform> RotateTransform for VisualizeAabbItem<T> {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.0.rotate_on_axis(axis, angle);
        self
    }
}

impl<T: Aabb + ScaleTransform> ScaleTransform for VisualizeAabbItem<T> {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.0.scale(scale);
        self
    }
}

impl<T: Aabb + Extract<Target = CoreItem>> Extract for VisualizeAabbItem<T> {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        self.0.extract_into(buf);

        let [min, max] = self.0.aabb();
        let size = max - min;
        let mut rect = VItem::from(Rectangle::new(size.x, size.y));
        rect.shift(min + size / 2.0);
        rect.set_stroke_color(color::palettes::manim::YELLOW_C);
        rect.set_fill_color(AlphaColor::TRANSPARENT);
        rect.extract_into(buf);
    }
}
