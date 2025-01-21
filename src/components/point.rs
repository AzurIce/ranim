use std::ops::{Deref, DerefMut};

use glam::Vec3;

#[derive(Debug, Clone)]
pub struct Points {
    inner: Vec<Vec3>,
}

// impl Alignable for Points {
//     fn align_with(&mut self, other: &mut Self) {

//     }
// }

impl Deref for Points {
    type Target = Vec<Vec3>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Points {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Points {
    pub fn new() -> Self {
        Self { inner: vec![] }
    }

    pub fn set(&mut self, points: impl IntoIterator<Item = Vec3>) {
        self.inner = points.into_iter().collect();
    }
}

#[cfg(test)]
mod test {
    use super::Points;

    #[test]
    fn test_points_new() {
        let mut points = Points::new();
        let _ = points.set([]);
        // let _ = points.set(&[]);
        let _ = points.set(vec![]);
        let _ = points.set(vec![].into_iter());
        // let _ = points.set(vec![].iter());
    }
}
