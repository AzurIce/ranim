use std::ops::{Deref, DerefMut};

use crate::prelude::Alignable;

pub mod point;
pub mod rgba;
pub mod vpoint;
pub mod width;

pub struct ComponentData<T>(Vec<T>);

impl<T> From<Vec<T>> for ComponentData<T> {
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}

impl<T> AsRef<Vec<T>> for ComponentData<T> {
    fn as_ref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T> AsMut<Vec<T>> for ComponentData<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}

impl<T> Deref for ComponentData<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ComponentData<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Alignable> Alignable for ComponentData<T> {
    fn is_aligned(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().zip(other.iter()).all(|(a, b)| a.is_aligned(b))
    }
    fn align_with(&mut self, other: &mut Self) {
        for (a, b) in self.iter_mut().zip(other.iter_mut()) {
            a.align_with(b);
        }
    }
}
