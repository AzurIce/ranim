use crate::rabject::{vmobject::VMobject, Interpolatable, RabjectWithId};

use super::{Animation, AnimationConfig, AnimationFunc};

pub struct Transform {
    aligned_source: RabjectWithId<VMobject>,
    aligned_target: RabjectWithId<VMobject>,
}

impl Transform {
    pub fn new(
        rabject: RabjectWithId<VMobject>,
        target: RabjectWithId<VMobject>,
    ) -> Animation<VMobject> {
        let mut aligned_source = rabject.clone();
        let mut aligned_target = target.clone();
        if !aligned_source.is_aligned(&aligned_target) {
            aligned_source.align_with(&mut aligned_target);
        }
        // trace!("[Transform::new] aligned_source: {:#?}", aligned_source.points());
        // trace!("[Transform::new] aligned_target: {:#?}", aligned_target.points());

        Animation::new(
            rabject,
            Self {
                aligned_source,
                aligned_target,
            },
        )
        .with_config(AnimationConfig {
            end_rabject: Some(target),
            ..AnimationConfig::default()
        })
    }
}

impl AnimationFunc<VMobject> for Transform {
    fn pre_anim(&mut self, rabject: &mut RabjectWithId<VMobject>) {
        rabject.set_points(self.aligned_source.points().to_vec());
    }

    fn interpolate(&mut self, rabject: &mut RabjectWithId<VMobject>, alpha: f32) {
        let points = self
            .aligned_source
            .points()
            .iter()
            .zip(self.aligned_target.points().iter())
            .map(|(p1, p2)| p1.lerp(p2, alpha))
            .collect();
        // trace!("[Transform::interpolate] t: {alpha} points: {:#?}", points);
        rabject.set_points(points);
    }

    fn post_anim(&mut self, rabject: &mut RabjectWithId<VMobject>) {
        rabject.set_points(self.aligned_target.points().to_vec());
    }
}
