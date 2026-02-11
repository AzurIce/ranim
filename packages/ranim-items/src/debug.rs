//! Debug utilities for visualizing item properties.

use std::ops::{Deref, DerefMut};

use ranim_core::{
    Extract,
    anchor::Aabb,
    color::{self, AlphaColor},
    core_item::CoreItem,
    glam::dvec2,
};

use crate::vitem::geometry::Rectangle;

/// Wrapper that visualizes the AABB of the inner item as a wireframe rectangle.
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

impl<T: Aabb + Extract<Target = CoreItem>> Extract for VisualizeAabbItem<T> {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        self.0.extract_into(buf);

        let [min, max] = self.0.aabb();
        let size = max - min;
        let mut rect = Rectangle::from_min_size(min, dvec2(size.x, size.y));
        rect.stroke_rgba = color::palettes::manim::YELLOW_C;
        rect.fill_rgba = AlphaColor::TRANSPARENT;
        rect.extract_into(buf);
    }
}
