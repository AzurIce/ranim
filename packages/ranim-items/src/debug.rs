//! Debug utilities for visualizing item properties.

use std::ops::{Deref, DerefMut};

use ranim_core::{
    Extract,
    anchor::{DBounds3, SemanticBounds},
    color::{self, AlphaColor},
    core_item::CoreItem,
    glam::{DVec3, dvec2},
    traits::{RotateTransform, Scale, ShiftTransform},
};

use crate::vitem::geometry::Rectangle;

/// Wrapper that visualizes the semantic bounds of the inner item as a wireframe rectangle.
#[derive(Clone)]
pub struct VisualizeSemanticBoundsItem<T: SemanticBounds>(pub T);

impl<T: SemanticBounds> Deref for VisualizeSemanticBoundsItem<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: SemanticBounds> DerefMut for VisualizeSemanticBoundsItem<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: SemanticBounds> SemanticBounds for VisualizeSemanticBoundsItem<T> {
    fn semantic_bounds(&self) -> DBounds3 {
        self.0.semantic_bounds()
    }
}

impl<T: SemanticBounds + ShiftTransform> ShiftTransform for VisualizeSemanticBoundsItem<T> {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.0.shift(offset);
        self
    }
}

impl<T: SemanticBounds + RotateTransform> RotateTransform for VisualizeSemanticBoundsItem<T> {
    fn rotate_on_axis(&mut self, axis: DVec3, angle: f64) -> &mut Self {
        self.0.rotate_on_axis(axis, angle);
        self
    }
}

impl<T: SemanticBounds + Scale> Scale for VisualizeSemanticBoundsItem<T> {
    fn scale(&mut self, scale: DVec3) -> &mut Self {
        self.0.scale(scale);
        self
    }
}

impl<T: SemanticBounds + Extract<Target = CoreItem>> Extract for VisualizeSemanticBoundsItem<T> {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        self.0.extract_into(buf);

        let bounds = self.0.semantic_bounds();
        let min = bounds.world_min();
        let size = bounds.world_size();
        let mut rect = Rectangle::from_min_size(min, dvec2(size.x, size.y));
        rect.stroke_rgba = color::palettes::manim::YELLOW_C;
        rect.fill_rgba = AlphaColor::TRANSPARENT;
        rect.extract_into(buf);
    }
}
